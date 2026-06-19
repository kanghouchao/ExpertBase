//! asr ドメイン層。転写エンジンのポート（trait）と境界の値オブジェクト、ドメインエラー。
//! 具体的な whisper.cpp / ONNX 実装には依存しない。

use std::path::PathBuf;

/// 転写言語。UI から渡る選択（自動/中文/日本語/英語）に対応する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
  /// 自動検出（whisper に言語ヒントを渡さない）。
  Auto,
  Zh,
  Ja,
  En,
}

impl Language {
  /// IPC で渡る言語コード文字列から解釈する。未知の値は自動検出にフォールバックする。
  pub fn from_code(code: &str) -> Language {
    match code.trim().to_ascii_lowercase().as_str() {
      "zh" => Language::Zh,
      "ja" => Language::Ja,
      "en" => Language::En,
      _ => Language::Auto,
    }
  }

  /// whisper に渡す言語コード。自動検出なら None（whisper 側が推定する）。
  pub fn to_whisper_code(&self) -> Option<&'static str> {
    match self {
      Language::Auto => None,
      Language::Zh => Some("zh"),
      Language::Ja => Some("ja"),
      Language::En => Some("en"),
    }
  }
}

/// 転写リクエスト。受信箱素材の添付 WAV パスと言語指定を運ぶ。
#[derive(Clone, Debug)]
pub struct TranscriptRequest {
  pub wav_path: PathBuf,
  pub language: Language,
}

/// 転写結果。
#[derive(Clone, Debug, PartialEq)]
pub struct Transcript {
  pub text: String,
  /// 自動検出時に判明した言語（whisper が返す）。指定時は None でよい。
  pub detected_language: Option<String>,
}

/// 転写エラー。UI で区別して表示できるようにする。
#[derive(Debug, PartialEq)]
pub enum TranscribeError {
  /// モデル未取得・ダウンロード失敗など。
  ModelUnavailable(String),
  /// 音声の読み込み/デコード失敗。
  Decode(String),
  /// 推論エンジンの失敗。
  Engine(String),
}

impl std::fmt::Display for TranscribeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TranscribeError::ModelUnavailable(m) => write!(f, "语音模型不可用: {m}"),
      TranscribeError::Decode(m) => write!(f, "音频解码失败: {m}"),
      TranscribeError::Engine(m) => write!(f, "转写引擎错误: {m}"),
    }
  }
}

/// 転写エンジン接合面。アプリケーション層はこの trait の裏でのみ転写を呼ぶ。
/// 将来の Nemotron（ストリーミング）/ SenseVoice は別実装として差し込む（下流は変更不要）。
pub trait TranscriptionEngine {
  fn transcribe(&self, req: TranscriptRequest) -> Result<Transcript, TranscribeError>;
}

/// テスト用の決定的エンジン（ネットワーク・ネイティブ依存なし）。
/// WAV ファイルが存在すれば固定文を返し、無ければデコードエラーにする。
#[cfg(test)]
pub struct FakeEngine {
  pub text: String,
}

#[cfg(test)]
impl TranscriptionEngine for FakeEngine {
  fn transcribe(&self, req: TranscriptRequest) -> Result<Transcript, TranscribeError> {
    if !req.wav_path.is_file() {
      return Err(TranscribeError::Decode("WAV が見つからない".into()));
    }
    Ok(Transcript { text: self.text.clone(), detected_language: None })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn language_from_code_maps_known_codes_and_falls_back_to_auto() {
    assert_eq!(Language::from_code("zh"), Language::Zh);
    assert_eq!(Language::from_code("JA"), Language::Ja);
    assert_eq!(Language::from_code(" en "), Language::En);
    assert_eq!(Language::from_code("auto"), Language::Auto);
    assert_eq!(Language::from_code("fr"), Language::Auto);
  }

  #[test]
  fn auto_language_passes_no_hint_to_whisper() {
    assert_eq!(Language::Auto.to_whisper_code(), None);
    assert_eq!(Language::Zh.to_whisper_code(), Some("zh"));
    assert_eq!(Language::Ja.to_whisper_code(), Some("ja"));
    assert_eq!(Language::En.to_whisper_code(), Some("en"));
  }

  #[test]
  fn transcribe_error_displays_distinct_messages() {
    assert_eq!(TranscribeError::Decode("x".into()).to_string(), "音频解码失败: x");
    assert_eq!(
      TranscribeError::ModelUnavailable("y".into()).to_string(),
      "语音模型不可用: y"
    );
  }
}
