//! ナレッジベース機能。DDD レイヤ構成（domain / application / infrastructure / interface）。

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

// kb 機能の公開 API（他機能・コマンド登録が参照する安定面）。
// 他機能はこの面のみを参照し、各レイヤ内部へ直接到達しない。
pub use domain::{entry, material};
pub use infrastructure::{index, store};
pub(crate) use application::open_active;
pub(crate) use domain::registry::checked_kb_markdown_path;
