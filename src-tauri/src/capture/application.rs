//! capture アプリケーション層。取り込みユースケース。
//! ドメイン（タイプ判定）とインフラ（抽出・FS）を編成し、受信箱へ素材を確定する。
//! すべての取り込みは AI なし・完全ローカル。

use std::path::Path;

use chrono::Utc;
use rusqlite::Connection;

use crate::capture::domain::{kind_for_ext, split_name};
use crate::capture::infrastructure::{doc, web};
use crate::kb::index;
use crate::kb::material::{serialize_material, Material, MaterialMeta};

/// 受信箱へ素材を 1 件書き出し、インデックスへ登録して相対パスを返す。
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

/// 録音バイト列を attachments/ へ WAV として保存し、audio/pending の素材を受信箱へ作る。
/// 本文は空（後段の転写が本文を埋める）。原始音声は外部送信しない。
pub fn write_audio_material(
  root: &Path,
  conn: &Connection,
  wav: &[u8],
  source: &str,
) -> Result<String, String> {
  let dir = root.join("attachments");
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let stamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
  let mut rel = format!("attachments/{stamp}.wav");
  let mut n = 2;
  while root.join(&rel).exists() {
    rel = format!("attachments/{stamp}-{n}.wav");
    n += 1;
  }
  std::fs::write(root.join(&rel), wav).map_err(|e| e.to_string())?;
  write_material(root, conn, "audio", source, "", Some(&rel))
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
pub fn ingest_text(home: &Path, content: &str, source: &str) -> Result<String, String> {
  let (root, conn) = crate::kb::open_active(home)?;
  write_material(&root, &conn, "text", source, content, None)
}

/// ローカルファイルを受信箱へ取り込む。
/// 文字を持つ素材（テキスト）は本文として抽出し、
/// 不透明メディア（音声/動画/画像）は attachments/ へコピーして添付参照にする。
/// PDF/Word は本文を抽出し、原本も添付として残す（出典参照のため）。
pub fn ingest_file(home: &Path, path: &str) -> Result<String, String> {
  let (root, conn) = crate::kb::open_active(home)?;
  let src = Path::new(path);
  let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("");
  let kind = kind_for_ext(ext);
  let source = src.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
  match kind {
    "text" => {
      let body = std::fs::read_to_string(src).map_err(|e| e.to_string())?;
      write_material(&root, &conn, "text", &source, &body, None)
    }
    "pdf" | "doc" => {
      let body = if kind == "pdf" {
        doc::extract_pdf(src)?
      } else {
        doc::extract_docx(src)?
      };
      let att = copy_attachment(&root, src)?;
      write_material(&root, &conn, kind, &source, &body, Some(&att))
    }
    _ => {
      let att = copy_attachment(&root, src)?;
      write_material(&root, &conn, kind, &source, "", Some(&att))
    }
  }
}

/// 録音バイト列（WAV）を受信箱へ取り込む。停止後に転写へ渡せる audio 素材を作る。
pub fn ingest_audio_bytes(home: &Path, wav: &[u8], source: &str) -> Result<String, String> {
  let (root, conn) = crate::kb::open_active(home)?;
  write_audio_material(&root, &conn, wav, source)
}

/// Web ページを取り込む。Rust 側で取得し、Readability で本文を抽出して受信箱へ保存する。
pub async fn ingest_web(home: &Path, url: &str) -> Result<String, String> {
  let html = web::fetch_html(url).await?;
  let (title, markdown) = web::extract_readable(&html, url)?;
  let body = if title.trim().is_empty() {
    markdown
  } else {
    format!("# {title}\n\n{markdown}")
  };
  let (root, conn) = crate::kb::open_active(home)?;
  write_material(&root, &conn, "web", url, &body, None)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::index;

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

  #[test]
  fn write_audio_material_saves_wav_and_creates_audio_inbox_item() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    let rel = write_audio_material(root, &conn, b"RIFF....WAVEfake", "recording").unwrap();
    assert!(root.join(&rel).is_file());
    assert!(rel.starts_with("inbox/"));
    // 録音バイト列が attachments/ に WAV として保存されている。
    let wavs: Vec<_> = std::fs::read_dir(root.join("attachments"))
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| e.path().extension().is_some_and(|x| x == "wav"))
      .collect();
    assert_eq!(wavs.len(), 1);
    // 受信箱には audio/pending の素材が 1 件できる（本文は空、後で転写が埋める）。
    let items = index::list_inbox(&conn).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, "audio");
    assert_eq!(items[0].status, "pending");
  }
}
