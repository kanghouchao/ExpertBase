mod ai;
mod capture;
mod kb;
mod workshop;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      kb::kb_list,
      kb::kb_create,
      kb::kb_set_active,
      kb::kb_rebuild_index,
      kb::kb_list_entries,
      kb::kb_search,
      kb::kb_backlinks,
      kb::kb_stats,
      kb::kb_graph,
      kb::kb_orphans,
      kb::kb_read_entry,
      kb::kb_save_entry,
      kb::kb_list_inbox,
      capture::capture_text,
      capture::capture_file,
      capture::capture_web,
      ai::ai_set_key,
      ai::ai_has_key,
      workshop::workshop_draft,
      workshop::workshop_confirm
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
