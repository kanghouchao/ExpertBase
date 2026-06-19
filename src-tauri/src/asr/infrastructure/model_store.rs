//! 転写モデルの取得層。初回利用時に ggml モデルをダウンロードし、sha256 で検証して
//! ローカルへキャッシュする。進捗は Tauri 非依存のコールバックで上報する（Channel への
//! 橋渡しは interface 層が行う）。

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::asr::domain::TranscribeError;

/// 取得対象モデルの仕様。
#[derive(Clone, Debug)]
pub struct ModelSpec {
  pub file_name: String,
  pub url: String,
  pub sha256: String,
}

/// 既定モデル: whisper.cpp 公式の large-v3-turbo q5_0（約 547MB）。
/// 実際に呼ぶのは whisper feature を有効にした interface のみ。
#[cfg(feature = "whisper")]
pub fn default_whisper_model() -> ModelSpec {
  ModelSpec {
    file_name: "ggml-large-v3-turbo-q5_0.bin".into(),
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin"
      .into(),
    sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2".into(),
  }
}

/// バイト列の sha256 を 16 進文字列で返す（テスト用ヘルパ）。
#[cfg(test)]
fn sha256_hex(bytes: &[u8]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  format!("{:x}", hasher.finalize())
}

/// ファイルの sha256 を逐次読み込みで計算する（大きいモデルでもメモリに載せない）。
fn file_sha256(path: &Path) -> std::io::Result<String> {
  let mut file = std::fs::File::open(path)?;
  let mut hasher = Sha256::new();
  let mut buf = [0u8; 65536];
  loop {
    let n = file.read(&mut buf)?;
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
  }
  Ok(format!("{:x}", hasher.finalize()))
}

/// 指定 spec のモデルが検証済みでキャッシュ済みか。
pub fn is_cached(dir: &Path, spec: &ModelSpec) -> bool {
  let dest = dir.join(&spec.file_name);
  match file_sha256(&dest) {
    Ok(hex) => hex == spec.sha256,
    Err(_) => false,
  }
}

/// モデルを確実に用意し、ローカルパスを返す。検証済みキャッシュがあればダウンロードしない（冪等）。
/// `progress(downloaded, total)` はダウンロード中のみ呼ばれる。
pub fn ensure_model(
  dir: &Path,
  spec: &ModelSpec,
  progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<PathBuf, TranscribeError> {
  let dest = dir.join(&spec.file_name);
  if is_cached(dir, spec) {
    return Ok(dest);
  }
  std::fs::create_dir_all(dir).map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
  download(&spec.url, &dest, progress)?;
  let got = file_sha256(&dest).map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
  if got != spec.sha256 {
    let _ = std::fs::remove_file(&dest);
    return Err(TranscribeError::ModelUnavailable(
      "下载文件校验失败（sha256 不匹配）".into(),
    ));
  }
  Ok(dest)
}

/// URL を逐次ダウンロードして dest へ書き出す。進捗を上報する。
fn download(
  url: &str,
  dest: &Path,
  progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), TranscribeError> {
  let client = reqwest::blocking::Client::builder()
    .timeout(None)
    .build()
    .map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
  let mut resp = client
    .get(url)
    .send()
    .map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?
    .error_for_status()
    .map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
  let total = resp.content_length();
  let mut file =
    std::fs::File::create(dest).map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
  let mut buf = [0u8; 65536];
  let mut downloaded = 0u64;
  loop {
    let n = resp
      .read(&mut buf)
      .map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
    if n == 0 {
      break;
    }
    file
      .write_all(&buf[..n])
      .map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
    downloaded += n as u64;
    progress(downloaded, total);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sha256_hex_matches_known_vector() {
    // "abc" の SHA-256 は既知のテストベクタ。
    assert_eq!(
      sha256_hex(b"abc"),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
  }

  #[test]
  fn is_cached_true_only_when_file_present_and_hash_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let content = b"fake-model-bytes";
    let spec = ModelSpec {
      file_name: "m.bin".into(),
      url: "https://example.invalid/m.bin".into(),
      sha256: sha256_hex(content),
    };
    // ファイルが無ければ false。
    assert!(!is_cached(dir, &spec));
    // 正しい内容を置けば true。
    std::fs::write(dir.join("m.bin"), content).unwrap();
    assert!(is_cached(dir, &spec));
    // 内容が変われば（ハッシュ不一致）false。
    std::fs::write(dir.join("m.bin"), b"tampered").unwrap();
    assert!(!is_cached(dir, &spec));
  }

  #[test]
  fn ensure_model_skips_download_when_cached() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let content = b"fake-model-bytes";
    let spec = ModelSpec {
      file_name: "m.bin".into(),
      url: "https://example.invalid/m.bin".into(),
      sha256: sha256_hex(content),
    };
    std::fs::write(dir.join("m.bin"), content).unwrap();

    // キャッシュ済みなのでダウンロード（progress）は一切呼ばれない。
    let mut called = false;
    let path = ensure_model(dir, &spec, &mut |_, _| called = true).unwrap();
    assert_eq!(path, dir.join("m.bin"));
    assert!(!called, "キャッシュ命中時はダウンロードしないはず");
  }
}
