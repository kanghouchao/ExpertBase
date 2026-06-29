//! 音声認識（ASR）機能 — 工坊への搬入待ちで休止中（コードはディスク上に残置）。
//! 録音 UI と inbox を廃止した時点で transcribe の呼び出し経路と inbox 契約が消えたため、
//! 各レイヤをコンパイル対象から外す。再導入は別 spec（録音 / 動画 / 臨時目录）で行う。
//! ponytail: mod 宣言だけ外す最小休止。再開時はこの 4 行のコメントを戻す。
// mod application;
// mod domain;
// mod infrastructure;
// pub mod interface;
