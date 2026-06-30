//! 抽出器の置き場（doc + web）。元 capture 機能は工坊へ畳み込み済み。

mod infrastructure;

// 文書テキスト抽出（PDF/Word）と Web 取得 / 本文抽出。
// workshop の read_source（外部ファイル）と fetch_web（URL）が再利用する。
pub use infrastructure::doc::{extract_docx, extract_pdf};
pub use infrastructure::web::{extract_readable, fetch_html};
