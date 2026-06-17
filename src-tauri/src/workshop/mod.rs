//! ワークショップ機能。DDD レイヤ構成（domain / application / interface）。
//! infrastructure 層は持たない（永続化・索引・AI は kb / ai のインフラを編成して使う）。

mod application;
mod domain;
pub mod interface;

// workshop 機能の公開面は interface のコマンドのみ（lib.rs で登録）。
// 各レイヤ内部は非公開とし、他機能から直接到達させない。
