//! ワークショップ機能。DDD レイヤ構成（application / infrastructure / interface）。
//! domain 層は持たない。infrastructure 層は Rig（AI フレームワーク）と KB を繋ぐアダプタ
//! （Tool 実装・エージェント駆動）を収める。永続化・索引は kb のインフラを編成して使う。

mod application;
mod infrastructure;
pub mod interface;

// workshop 機能の公開面は interface のコマンドのみ（lib.rs で登録）。
// 各レイヤ内部は非公開とし、他機能から直接到達させない。
