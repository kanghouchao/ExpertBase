//! workshop インフラ層: 外部フレームワーク（Rig）と KB を繋ぐアダプタ。
//! tools は Rig の Tool 実装（KB 操作）。rig_agent（step 3）は Ollama エージェントの駆動。

pub(crate) mod history;
mod tools;
pub(crate) mod rig_agent;
