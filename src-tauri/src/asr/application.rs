//! asr アプリケーション層。転写ユースケース。
//! 受信箱の audio 素材を読み、エンジン（ポート）で転写し、本文へ書き戻して受信箱を更新する。
//! Tauri/ダウンロード/進捗には依存しない（それらは interface / infrastructure の責務）。

use std::path::Path;

use rusqlite::Connection;

use crate::asr::domain::{Language, TranscribeError, TranscriptRequest, TranscriptionEngine};
use crate::kb::index;
use crate::kb::material::parse_material;

/// 受信箱の audio 素材を転写し、本文へ書き戻す（テスト可能なコア）。
/// 添付 WAV をエンジンへ渡し、得たテキストを素材本文に入れ、status を "transcribed" にして再索引する。
/// 転写後のテキストを返す。
pub fn transcribe_into_material(
  root: &Path,
  conn: &Connection,
  inbox_path: &str,
  language: Language,
  engine: &impl TranscriptionEngine,
) -> Result<String, String> {
  let abs = root.join(inbox_path);
  let raw = std::fs::read_to_string(&abs).map_err(|e| e.to_string())?;
  let mut material = parse_material(&raw)?;
  if material.meta.attachment.trim().is_empty() {
    return Err(TranscribeError::Decode("素材に音声添付がない".into()).to_string());
  }
  let wav_path = root.join(&material.meta.attachment);
  let transcript = engine
    .transcribe(TranscriptRequest { wav_path, language })
    .map_err(|e| e.to_string())?;

  material.body = transcript.text.clone();
  material.meta.status = "transcribed".to_string();
  std::fs::write(&abs, crate::kb::material::serialize_material(&material)?)
    .map_err(|e| e.to_string())?;
  index::upsert_inbox(
    conn,
    inbox_path,
    &material.meta.kind,
    &material.meta.source,
    "transcribed",
    &material.meta.captured_at,
  )?;
  Ok(transcript.text)
}

/// 受信箱の audio 素材を転写する（アクティブ KB を開いてコアへ委譲）。
/// 実際に呼ぶのは whisper feature を有効にした interface のみ。テストはコアを直接叩く。
#[cfg(feature = "whisper")]
pub fn transcribe_material(
  home: &Path,
  inbox_path: &str,
  language: Language,
  engine: &impl TranscriptionEngine,
) -> Result<String, String> {
  let (root, conn) = crate::kb::open_active(home)?;
  transcribe_into_material(&root, &conn, inbox_path, language, engine)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::asr::domain::FakeEngine;
  use crate::kb::index;
  use crate::kb::material::{serialize_material, Material, MaterialMeta};

  /// 受信箱に audio/pending 素材（添付 WAV 付き）を 1 件用意する。
  fn seed_audio_material(root: &Path, conn: &Connection) -> String {
    std::fs::create_dir_all(root.join("attachments")).unwrap();
    std::fs::write(root.join("attachments/rec.wav"), b"RIFF....WAVEfake").unwrap();
    std::fs::create_dir_all(root.join("inbox")).unwrap();
    let material = Material {
      meta: MaterialMeta {
        kind: "audio".into(),
        source: "recording".into(),
        status: "pending".into(),
        attachment: "attachments/rec.wav".into(),
        captured_at: "2026-06-17T00:00:00Z".into(),
      },
      body: String::new(),
    };
    let rel = "inbox/rec.md";
    std::fs::write(root.join(rel), serialize_material(&material).unwrap()).unwrap();
    index::upsert_inbox(conn, rel, "audio", "recording", "pending", "2026-06-17T00:00:00Z").unwrap();
    rel.to_string()
  }

  #[test]
  fn transcribe_fills_material_body_and_marks_transcribed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    let inbox_path = seed_audio_material(root, &conn);

    let text = transcribe_into_material(
      root,
      &conn,
      &inbox_path,
      Language::Auto,
      &FakeEngine { text: "会議メモ".into() },
    )
    .unwrap();
    assert_eq!(text, "会議メモ");

    // 素材本文に転写が書き戻され、status が transcribed になる。
    let raw = std::fs::read_to_string(root.join(&inbox_path)).unwrap();
    let m = parse_material(&raw).unwrap();
    assert_eq!(m.body.trim(), "会議メモ");
    assert_eq!(m.meta.status, "transcribed");

    // インデックスにも transcribed が反映される。
    let items = index::list_inbox(&conn).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, "transcribed");
  }

  #[test]
  fn transcribe_errors_when_material_has_no_attachment() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    std::fs::create_dir_all(root.join("inbox")).unwrap();
    let material = Material {
      meta: MaterialMeta {
        kind: "text".into(),
        source: "manual".into(),
        status: "pending".into(),
        attachment: String::new(),
        captured_at: "2026-06-17T00:00:00Z".into(),
      },
      body: "ただのメモ".into(),
    };
    std::fs::write(root.join("inbox/note.md"), serialize_material(&material).unwrap()).unwrap();

    let err = transcribe_into_material(
      root,
      &conn,
      "inbox/note.md",
      Language::Auto,
      &FakeEngine { text: "x".into() },
    )
    .unwrap_err();
    assert!(err.contains("音声添付がない"));
  }
}
