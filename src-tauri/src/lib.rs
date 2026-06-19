mod ai;
mod asr;
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
      kb::interface::kb_list,
      kb::interface::kb_create,
      kb::interface::kb_set_active,
      kb::interface::kb_rebuild_index,
      kb::interface::kb_list_entries,
      kb::interface::kb_search,
      kb::interface::kb_backlinks,
      kb::interface::kb_stats,
      kb::interface::kb_graph,
      kb::interface::kb_orphans,
      kb::interface::kb_read_entry,
      kb::interface::kb_read_inbox_material,
      kb::interface::kb_save_entry,
      kb::interface::kb_delete_inbox_material,
      kb::interface::kb_list_inbox,
      capture::interface::capture_text,
      capture::interface::capture_file,
      capture::interface::capture_audio,
      capture::interface::capture_web,
      ai::interface::ai_has_key,
      ai::interface::ai_list_ollama_models,
      workshop::interface::workshop_draft,
      workshop::interface::workshop_confirm,
      asr::interface::transcribe_material
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
