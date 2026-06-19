//! asr インターフェイス層。Tauri コマンド（IPC アダプタ）。
//! 実際の転写は `whisper` feature を有効にしたビルドでのみ動作する（cmake が必要）。

use serde::Serialize;
use tauri::ipc::Channel;

/// モデルのダウンロード進捗。フロントの Channel へ送る。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
  pub downloaded: u64,
  pub total: Option<u64>,
}

/// 受信箱の audio 素材を転写し、本文へ書き戻して受信箱を更新する。
/// 初回はモデルをダウンロードし、進捗を `on_progress` で上報する。
/// 転写後のテキストを返す。
#[tauri::command]
pub async fn transcribe_material(
  app: tauri::AppHandle,
  inbox_path: String,
  language: String,
  on_progress: Channel<DownloadProgress>,
) -> Result<String, String> {
  #[cfg(not(feature = "whisper"))]
  {
    let _ = (&app, &inbox_path, &language, &on_progress);
    return Err("本构建未启用语音转写（请以 --features whisper 重新构建）".into());
  }
  #[cfg(feature = "whisper")]
  {
    use tauri::Manager;

    use crate::asr::application;
    use crate::asr::domain::Language;
    use crate::asr::infrastructure::{model_store, whisper::WhisperEngine};

    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let model_dir = app
      .path()
      .app_data_dir()
      .map_err(|e| e.to_string())?
      .join("models")
      .join("asr");
    let lang = Language::from_code(&language);

    // モデル取得（ダウンロード）と転写は CPU/IO 重め。UI スレッドを塞がないよう別スレッドへ。
    let joined = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
      let spec = model_store::default_whisper_model();
      let model_path = model_store::ensure_model(&model_dir, &spec, &mut |downloaded, total| {
        let _ = on_progress.send(DownloadProgress { downloaded, total });
      })
      .map_err(|e| e.to_string())?;
      let engine = WhisperEngine::load(&model_path).map_err(|e| e.to_string())?;
      application::transcribe_material(&home, &inbox_path, lang, &engine)
    })
    .await;

    match joined {
      Ok(inner) => inner,
      Err(e) => Err(e.to_string()),
    }
  }
}
