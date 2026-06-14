//! AI 機能。DDD レイヤ構成（domain ポート / infrastructure アダプタ / interface）。
//! application 層は持たない（AI を編成するユースケースは workshop が担う）。

pub mod domain;
pub mod infrastructure;
pub mod interface;

// ai 機能の公開 API（workshop 等が参照する安定面）。
pub use domain::{AiError, AiProvider, EntrySummary, StructureRequest, StructureResult};
pub use infrastructure::ollama;
#[cfg(test)]
pub use domain::FakeProvider;
