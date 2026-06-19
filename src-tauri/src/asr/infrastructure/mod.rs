//! asr インフラ層。モデルの取得（ダウンロード/キャッシュ/検証）と推論エンジンの具体実装。

pub mod model_store;
#[cfg(feature = "whisper")]
pub mod whisper;
