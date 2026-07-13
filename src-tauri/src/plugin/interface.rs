//! plugin インターフェイス層。Tauri コマンド（IPC アダプタ）。

use tauri::Manager;

use crate::error::AppError;

use super::{discover_skills, Skill};

/// 発見済み技能一覧（KB `skills/` + `~/.agents/skills/`、同名は KB 側が勝つ）を返す。
/// 常駐状態・キャッシュは持たない＝呼び出しごとに毎回走査する。アクティブ KB が無くても
/// （設定ダイアログを KB 未選択で開いた場合など）エラーにせず user 側だけ返す。
#[tauri::command]
pub async fn plugin_list_skills(app: tauri::AppHandle) -> Result<Vec<Skill>, AppError> {
  let home = app.path().home_dir().map_err(AppError::generic)?;
  tauri::async_runtime::spawn_blocking(move || -> Result<Vec<Skill>, AppError> {
    let root = crate::kb::active_kb_root(&home).ok();
    Ok(discover_skills(root.as_deref(), &home))
  })
  .await
  .map_err(AppError::generic)?
}
