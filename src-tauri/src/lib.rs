mod agent;
mod error;
mod extract;
mod kb;
mod workshop;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(workshop::interface::WorkshopCancel::default())
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
      kb::interface::kb_list,
      kb::interface::kb_create,
      kb::interface::kb_set_active,
      kb::interface::kb_delete,
      kb::interface::kb_rebuild_index,
      kb::interface::kb_list_entries,
      kb::interface::kb_search,
      kb::interface::kb_backlinks,
      kb::interface::kb_stats,
      kb::interface::kb_graph,
      kb::interface::kb_orphans,
      kb::interface::kb_read_entry,
      kb::interface::kb_save_entry,
      agent::interface::ai_has_key,
      agent::interface::ai_list_ollama_models,
      agent::interface::ai_list_models,
      agent::interface::ai_get_settings,
      agent::interface::ai_set_settings,
      workshop::interface::workshop_chat,
      workshop::interface::workshop_get_conversation,
      workshop::interface::workshop_list_conversations,
      workshop::interface::workshop_save_conversation,
      workshop::interface::workshop_cancel
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
