//! AI 機能。DDD レイヤ構成（domain ポート / infrastructure アダプタ / interface）。
//! application 層は持たない（AI を編成するユースケースは workshop が担う）。
//! agent: プロバイダ非依存の指示層（プロンプト + 出力スキーマ）。transport から切り離す。

pub mod agent;
mod domain;
mod infrastructure;
pub mod interface;

// ai 機能の公開 API（workshop 等が参照する安定面）。
// 他機能はこの面のみを参照し、各レイヤ内部へ直接到達しない。
pub use domain::{
  AiError, AiProvider, ChatTurn, EntrySummary, StreamProgress, StructureRequest, StructureResult,
};
pub use infrastructure::ollama;
#[cfg(test)]
pub use domain::FakeProvider;
