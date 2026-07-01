//! 汎用 agent 機能（ブラックボックス）。DDD レイヤ構成（domain / infrastructure / interface）。
//! 業務（kb / workshop）に依存せず、注入されたツール集でツールループと推論を回す。
//! 指示層（system プロンプト）は業務固有なので呼び出し側（workshop）が持つ。

mod domain;
mod infrastructure;
pub mod interface;

// agent 機能の公開 API（workshop 等が参照する安定面）。
// 他機能はこの面のみを参照し、各レイヤ内部へ直接到達しない。
pub use domain::{resolve_base_url, AiError, ChatTurn, Provider, StreamProgress};
pub use infrastructure::runner::run;
pub use infrastructure::settings_store;
