//! プラグイン化機能（Agent Skills）。DDD レイヤ構成（domain / infrastructure / interface）。
//! MCP（#42）も将来同居する、業務非依存の外部標準アダプタ。workshop はこの公開面だけを消費する。

mod domain;
mod infrastructure;
pub mod interface;

// plugin 機能の公開 API（workshop 等が参照する安定面）。
// 各レイヤ内部は非公開とし、他機能から直接到達させない。
pub(crate) use domain::{render_activated, render_catalog, Skill, SkillSource};
pub(crate) use infrastructure::activate_skill::ActivateSkill;
pub(crate) use infrastructure::scan::discover_skills;
