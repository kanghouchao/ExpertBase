//! ワークショップ機能。DDD レイヤ構成（domain / application / infrastructure / interface）。
//! infrastructure 層は Rig（AI フレームワーク）と KB を繋ぐアダプタ、および対話履歴の
//! 永続化を収める。

mod application;
mod domain;
mod infrastructure;
pub mod interface;

// workshop 機能の公開面は interface のコマンドのみ（lib.rs で登録）。
// 各レイヤ内部は非公開とし、他機能から直接到達させない。
