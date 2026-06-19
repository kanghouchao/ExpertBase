//! 音声認識（ASR）機能。録音した音声を文字へ転写する。
//! DDD レイヤ構成（domain / application / infrastructure / interface）。
//! 転写エンジンは domain のポート（trait）の裏に隠し、whisper.cpp 等の実装を差し替え可能にする。

mod application;
mod domain;
mod infrastructure;
pub mod interface;
