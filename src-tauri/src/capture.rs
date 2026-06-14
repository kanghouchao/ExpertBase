use std::path::Path;

use chrono::Utc;
use rusqlite::Connection;
use tauri::Manager;

use crate::kb::index;
use crate::kb::material::{serialize_material, Material, MaterialMeta};

pub mod doc;
pub mod web;

/// 拡張子から素材タイプを判定する。未知は "file"。
pub fn kind_for_ext(ext: &str) -> &'static str {
  match ext.to_ascii_lowercase().as_str() {
    "md" | "markdown" | "txt" | "text" => "text",
    "pdf" => "pdf",
    "doc" | "docx" => "doc",
    "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" => "audio",
    "mp4" | "mov" | "mkv" | "webm" | "avi" => "video",
    "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => "image",
    _ => "file",
  }
}

/// 名前を (ステム, 拡張子) に分割する。
fn split_name(name: &str) -> (&str, Option<&str>) {
  match name.rsplit_once('.') {
    Some((stem, ext)) if !stem.is_empty() => (stem, Some(ext)),
    _ => (name, None),
  }
}

/// 受信箱へ素材を 1 件書き出し、インデックスへ登録して相対パスを返す。
/// すべての取り込みはこの 1 関数を通る（AI なし・完全ローカル）。
pub fn write_material(
  root: &Path,
  conn: &Connection,
  kind: &str,
  source: &str,
  body: &str,
  attachment: Option<&str>,
) -> Result<String, String> {
  let dir = root.join("inbox");
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let captured_at = Utc::now().to_rfc3339();
  let stamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
  let mut rel = format!("inbox/{stamp}.md");
  let mut n = 2;
  while root.join(&rel).exists() {
    rel = format!("inbox/{stamp}-{n}.md");
    n += 1;
  }
  let material = Material {
    meta: MaterialMeta {
      kind: kind.to_string(),
      source: source.to_string(),
      status: "pending".to_string(),
      attachment: attachment.unwrap_or("").to_string(),
      captured_at: captured_at.clone(),
    },
    body: body.to_string(),
  };
  std::fs::write(root.join(&rel), serialize_material(&material)?).map_err(|e| e.to_string())?;
  index::upsert_inbox(conn, &rel, kind, source, "pending", &captured_at)?;
  Ok(rel)
}

/// ファイルを attachments/ へコピーし、相対パスを返す。原始メディアは外部送信しない。
pub fn copy_attachment(root: &Path, src: &Path) -> Result<String, String> {
  let dir = root.join("attachments");
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let name = src.file_name().ok_or("无效的文件名")?.to_string_lossy().to_string();
  let mut rel = format!("attachments/{name}");
  let mut n = 2;
  while root.join(&rel).exists() {
    let (stem, ext) = split_name(&name);
    rel = match ext {
      Some(e) => format!("attachments/{stem}-{n}.{e}"),
      None => format!("attachments/{stem}-{n}"),
    };
    n += 1;
  }
  std::fs::copy(src, root.join(&rel)).map_err(|e| e.to_string())?;
  Ok(rel)
}

/// テキスト/Markdown の貼り付けを受信箱へ取り込む。
#[tauri::command]
pub fn capture_text(
  app: tauri::AppHandle,
  content: String,
  source: String,
) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  write_material(&root, &conn, "text", &source, &content, None)
}

/// ローカルファイルを受信箱へ取り込む。
/// 文字を持つ素材（テキスト）は本文として抽出し、
/// 不透明メディア（音声/動画/画像）は attachments/ へコピーして添付参照にする。
/// PDF/Word の本文抽出は capture/doc.rs（Task 2.2）で実装する。
#[tauri::command]
pub fn capture_file(app: tauri::AppHandle, path: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  let src = Path::new(&path);
  let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("");
  let kind = kind_for_ext(ext);
  let source = src.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
  match kind {
    "text" => {
      let body = std::fs::read_to_string(src).map_err(|e| e.to_string())?;
      write_material(&root, &conn, "text", &source, &body, None)
    }
    "pdf" | "doc" => {
      // デジタル文書は本文を抽出し、原本も添付として残す（出典参照のため）。
      let body = if kind == "pdf" {
        doc::extract_pdf(src)?
      } else {
        doc::extract_docx(src)?
      };
      let att = copy_attachment(&root, src)?;
      write_material(&root, &conn, kind, &source, &body, Some(&att))
    }
    _ => {
      // 音声/動画/画像など不透明メディアは添付として保存（本文は任意で空）。
      let att = copy_attachment(&root, src)?;
      write_material(&root, &conn, kind, &source, "", Some(&att))
    }
  }
}

/// Web ページを取り込む。Rust 側で取得し、Readability で本文を抽出して受信箱へ保存する。
/// 原始 HTML はクラウドへ送らない（取得は HTTPS、抽出はローカル）。
#[tauri::command]
pub async fn capture_web(app: tauri::AppHandle, url: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let html = reqwest::Client::new()
    .get(&url)
    .header("User-Agent", "ExpertBase/0.1 (+local capture)")
    .send()
    .await
    .map_err(|e| e.to_string())?
    .text()
    .await
    .map_err(|e| e.to_string())?;
  let (title, markdown) = web::extract_readable(&html, &url)?;
  let body = if title.trim().is_empty() {
    markdown
  } else {
    format!("# {title}\n\n{markdown}")
  };
  let (root, conn) = crate::kb::open_active(&home)?;
  write_material(&root, &conn, "web", &url, &body, None)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn kind_for_ext_maps_known_extensions() {
    assert_eq!(kind_for_ext("pdf"), "pdf");
    assert_eq!(kind_for_ext("PNG"), "image");
    assert_eq!(kind_for_ext("mp3"), "audio");
    assert_eq!(kind_for_ext("md"), "text");
    assert_eq!(kind_for_ext("xyz"), "file");
  }

  #[test]
  fn write_text_material_creates_inbox_file_and_indexes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    let rel = write_material(root, &conn, "text", "paste", "メモ本文", None).unwrap();
    assert!(root.join(&rel).is_file());
    assert!(rel.starts_with("inbox/"));
    let items = index::list_inbox(&conn).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, "text");
    assert_eq!(items[0].status, "pending");
  }

  #[test]
  fn copy_attachment_copies_into_attachments_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let src = tmp.path().join("clip.png");
    std::fs::write(&src, b"PNG").unwrap();
    let rel = copy_attachment(root, &src).unwrap();
    assert!(root.join(&rel).is_file());
    assert!(rel.starts_with("attachments/"));
  }
}
