//! ワークショップ機能。DDD レイヤ構成（domain / application / interface）。
//! infrastructure 層は持たない（永続化・索引・AI は kb / ai のインフラを編成して使う）。

pub mod application;
pub mod domain;
pub mod interface;
