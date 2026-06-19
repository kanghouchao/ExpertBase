//! 音声認識（ASR）機能。録音した音声を文字へ転写する。
//! DDD レイヤ構成（domain / application / infrastructure / interface）。
//! 転写エンジンは domain のポート（trait）の裏に隠し、whisper.cpp 等の実装を差し替え可能にする。

// 転写エンジンの中身は whisper feature を有効にしたビルドでのみ使う。
// feature オフ時は interface のコマンドがエラーを返すだけなので、これらは
// テスト時を除き未使用（dead_code）になる。実際に使う構成だけコンパイルする。
#[cfg(any(feature = "whisper", test))]
mod application;
#[cfg(any(feature = "whisper", test))]
mod domain;
#[cfg(any(feature = "whisper", test))]
mod infrastructure;
pub mod interface;
