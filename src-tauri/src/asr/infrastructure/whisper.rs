//! whisper.cpp（whisper-rs）による転写エンジン実装。`whisper` feature でのみコンパイルする。
//! ネイティブビルドに cmake と C/C++ ツールチェインが必要。

use std::path::Path;

use whisper_rs::{
  convert_integer_to_float_audio, convert_stereo_to_mono_audio, FullParams, SamplingStrategy,
  WhisperContext, WhisperContextParameters,
};

use crate::asr::domain::{TranscribeError, Transcript, TranscriptRequest, TranscriptionEngine};

/// ロード済みの whisper モデルを保持する転写エンジン。
pub struct WhisperEngine {
  ctx: WhisperContext,
}

impl WhisperEngine {
  /// ggml モデルファイルを読み込む。
  pub fn load(model_path: &Path) -> Result<Self, TranscribeError> {
    let path = model_path
      .to_str()
      .ok_or_else(|| TranscribeError::ModelUnavailable("模型路径无效".into()))?;
    let ctx = WhisperContext::new_with_params(path, WhisperContextParameters::default())
      .map_err(|e| TranscribeError::ModelUnavailable(e.to_string()))?;
    Ok(Self { ctx })
  }
}

impl TranscriptionEngine for WhisperEngine {
  fn transcribe(&self, req: TranscriptRequest) -> Result<Transcript, TranscribeError> {
    let audio = read_wav_16k_mono(&req.wav_path)?;

    let mut state = self
      .ctx
      .create_state()
      .map_err(|e| TranscribeError::Engine(e.to_string()))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    // "auto" は自動検出して転写まで進む。set_detect_language(true) は検出のみで
    // 即 return し本文が空になる（whisper.cpp の仕様）ため使わない。
    params.set_language(Some(req.language.to_whisper_code().unwrap_or("auto")));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state
      .full(params, &audio)
      .map_err(|e| TranscribeError::Engine(e.to_string()))?;

    // whisper-rs 0.16: full_n_segments は c_int を直接返し、本文は WhisperSegment 経由で取る。
    let mut text = String::new();
    for i in 0..state.full_n_segments() {
      if let Some(seg) = state.get_segment(i) {
        let part = seg
          .to_str_lossy()
          .map_err(|e| TranscribeError::Engine(e.to_string()))?;
        text.push_str(&part);
      }
    }

    Ok(Transcript { text: text.trim().to_string(), detected_language: None })
  }
}

/// WAV を whisper が要求する 16kHz・モノラル・f32 へ読み込む。
fn read_wav_16k_mono(path: &Path) -> Result<Vec<f32>, TranscribeError> {
  let reader = hound::WavReader::open(path).map_err(|e| TranscribeError::Decode(e.to_string()))?;
  let spec = reader.spec();
  if spec.sample_rate != 16000 {
    return Err(TranscribeError::Decode(format!(
      "需要 16kHz 音频，实际为 {}Hz",
      spec.sample_rate
    )));
  }
  let samples: Vec<i16> = reader
    .into_samples::<i16>()
    .collect::<Result<_, _>>()
    .map_err(|e| TranscribeError::Decode(e.to_string()))?;
  let mut audio = vec![0.0f32; samples.len()];
  convert_integer_to_float_audio(&samples, &mut audio)
    .map_err(|e| TranscribeError::Decode(e.to_string()))?;
  if spec.channels == 2 {
    // 0.16 の API は出力バッファ（入力の半分の長さ）を受け取る。
    let mut mono = vec![0.0f32; audio.len() / 2];
    convert_stereo_to_mono_audio(&audio, &mut mono)
      .map_err(|e| TranscribeError::Decode(e.to_string()))?;
    audio = mono;
  }
  Ok(audio)
}
