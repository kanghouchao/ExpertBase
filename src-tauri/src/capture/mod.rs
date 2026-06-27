//! 取り込み機能。DDD レイヤ構成（domain / application / infrastructure / interface）。

mod application;
mod domain;
mod infrastructure;
pub mod interface;

// capture 機能の公開面は interface のコマンドのみ（lib.rs で登録）。
// 各レイヤ内部は非公開とし、他機能から直接到達させない。

// 例外: 文書テキスト抽出は workshop の read_source ツールが外部ファイル素材で再利用する。
pub use infrastructure::doc::{extract_docx, extract_pdf};
