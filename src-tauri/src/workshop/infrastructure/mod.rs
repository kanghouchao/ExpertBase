//! workshop インフラ層: 外部フレームワーク（Rig）と KB を繋ぐアダプタ。
//! tools は Rig の Tool 実装（KB 操作）と、汎用 agent へ注入するツール一式の組み立て。

pub(crate) mod history;
pub(crate) mod tools;
