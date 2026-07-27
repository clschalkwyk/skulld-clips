mod commands;
mod domain;
mod ffmpeg;
mod security;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(security::path_policy::PathPolicy::default())
        .manage(services::export::ExportRegistry::default())
        .on_webview_event(|webview, event| {
            if let tauri::WebviewEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                use tauri::Manager;

                let policy = webview
                    .app_handle()
                    .state::<security::path_policy::PathPolicy>();
                for path in paths {
                    let _ = policy.authorize_existing_file(path);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::assets::select_overlay_file,
            commands::assets::import_overlay_asset,
            commands::assets::write_caption_asset,
            commands::export::select_export_destination,
            commands::export::validate_export,
            commands::export::start_export,
            commands::export::cancel_export,
            commands::projects::select_media_file,
            commands::projects::select_project_file,
            commands::projects::select_projects_folder,
            commands::projects::probe_media,
            commands::projects::create_project,
            commands::projects::load_project,
            commands::projects::save_project,
            commands::projects::relink_source,
            commands::projects::list_recent_projects,
            commands::projects::remove_recent_project,
            commands::runtime::get_runtime_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running Skull’d Clip Forge");
}
