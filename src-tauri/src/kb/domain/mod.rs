//! kb ドメイン層。実体・値オブジェクト・不変条件・純関数のみ。
//! 永続化 / FS / Tauri / IPC DTO に依存しない。

pub mod entry;
// material は asr（音声認識）専用の素材モデル。inbox 廃止で唯一の利用者だった asr が休止したため、
// コンパイル対象から外して休止する（ソースはディスク上に残置）。asr 再導入時に asr/mod.rs と一緒に戻す。
// ponytail: mod 宣言だけ外す最小休止。[[asr/mod.rs]] と対で扱う。
// pub mod material;
pub mod registry;
