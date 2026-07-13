//! workshop インフラ: 添付素材を id で読む read_source ツール。

use std::convert::Infallible;
use std::path::Path;

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use unicode_normalization::UnicodeNormalization;

use crate::extract::{extract_docx, extract_pdf};

use super::{remember_source, UsedSources};

/// read_source の引数。id（素材識別子）を緩く受ける。
#[derive(Deserialize)]
pub struct ReadArgs {
  #[serde(default)]
  id: String,
}

/// 添付素材を id で読む読み取りツール（外部絶対パスのローカルファイルのみ）。
/// sources は許可された素材 id の集合＝モデルが任意のパスを読むのを防ぐ。
/// 読み取りのみ・KB へ落とさない。
pub struct ReadSource {
  pub sources: Vec<String>,
  pub used_sources: UsedSources,
}

impl Tool for ReadSource {
  const NAME: &'static str = "read_source";
  type Error = Infallible;
  type Args = ReadArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Read the full text of an attached source material by its id (see the # Sources list). Read a source before translating, rewriting, summarizing, or answering questions about it. Do not summarize or rewrite a source unless the user asks."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "id": { "type": "string", "description": "Source id from the # Sources list" }
        },
        "required": ["id"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let sources = self.sources.clone();
    let used_sources = self.used_sources.clone();
    let out = tokio::task::spawn_blocking(move || read_blocking(&sources, &used_sources, &args.id))
      .await
      .unwrap_or_else(|e| format!("(read task failed: {e})"));
    Ok(out)
  }
}

/// 素材読み取り（ブロッキング）。id を許可集合で検証してから、拡張子で抽出器を選ぶ。
/// source は外部絶対パスのみ（pdf/docx は抽出、その他はテキスト読み）。
/// エラーは全てモデル向け文字列で返す（ループ継続）。読み取りのみ・KB へ落とさない。
fn read_blocking(sources: &[String], used_sources: &UsedSources, id: &str) -> String {
  let id = id.trim();
  if id.is_empty() {
    return "(read_source needs a non-empty id)".to_string();
  }
  // 許可された素材だけ読む（モデルが任意パスを読むのを防ぐ）。
  // macOS はファイル名を NFD で返し、モデルは NFC で打ち直すため、Unicode 正規化（NFC）で照合する
  // （バイト一致だと日本語名の素材を必ず取りこぼす）。読み取りは検証済みの保存側パスで開く。
  let want: String = id.nfc().collect();
  let Some(source) = sources.iter().find(|s| s.nfc().collect::<String>() == want) else {
    return format!("(unknown source id: {id})");
  };
  let path = Path::new(source);
  let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
  let text = match ext.as_str() {
    "pdf" => extract_pdf(path),
    "docx" => extract_docx(path),
    _ => std::fs::read_to_string(path).map_err(|e| e.to_string()),
  }
  .map_err(|e| format!("read error: {e}"));
  match text {
    Ok(body) => {
      remember_source(used_sources, source);
      if body.trim().is_empty() {
        format!("(source {id} is empty)")
      } else {
        body
      }
    }
    Err(e) => format!("({e})"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};

  #[tokio::test]
  async fn read_source_reads_external_local_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("外部メモ.md");
    std::fs::write(&file, "外部ファイルの内容").unwrap();
    let id = file.to_string_lossy().to_string();

    let used_sources = Arc::new(Mutex::new(Vec::new()));
    let tool = ReadSource { sources: vec![id.clone()], used_sources: used_sources.clone() };
    let out = tool.call(ReadArgs { id }).await.unwrap();

    assert!(out.contains("外部ファイルの内容"));
    assert_eq!(*used_sources.lock().unwrap(), vec![file.to_string_lossy().to_string()]);
  }

  #[tokio::test]
  async fn read_source_matches_across_unicode_normalization() {
    // macOS のファイルダイアログはファイル名を NFD（分解）で返すが、モデルは tool 引数を
    // NFC（合成）で打ち直す。バイト一致だと日本語名の素材を必ず取りこぼすため、正規化して照合する。
    let tmp = tempfile::tempdir().unwrap();
    let nfd_name: String = "五分プレゼン.md".nfd().collect();
    let file = tmp.path().join(&nfd_name);
    std::fs::write(&file, "テキスト本文").unwrap();
    // sources にはディスク由来の NFD パス、モデルが渡すのは NFC パス。
    let nfd_id = file.to_string_lossy().to_string();
    let nfc_id: String = nfd_id.nfc().collect();
    assert_ne!(nfd_id, nfc_id, "前提: NFD と NFC でバイトが異なる");

    let tool = ReadSource {
      sources: vec![nfd_id],
      used_sources: Arc::new(Mutex::new(Vec::new())),
    };
    let out = tool.call(ReadArgs { id: nfc_id }).await.unwrap();

    assert!(out.contains("テキスト本文"), "was: {out}");
  }

  #[tokio::test]
  async fn read_source_rejects_unknown_id() {
    let used_sources = Arc::new(Mutex::new(Vec::new()));
    let tool = ReadSource { sources: vec![], used_sources: used_sources.clone() };
    let out = tool.call(ReadArgs { id: "/abs/secret.md".into() }).await.unwrap();
    assert!(out.contains("unknown source id"));
    assert!(used_sources.lock().unwrap().is_empty());
  }
}
