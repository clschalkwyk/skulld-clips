mod commands;
mod domain;
mod ffmpeg;
mod security;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(security::path_policy::PathPolicy::default())
        .manage(services::export::ExportRegistry::default())
        .setup(|app| {
            use tauri::Manager;

            let log_dir = app.path().app_log_dir()?;
            app.manage(services::diagnostics::Diagnostics::new(log_dir));
            let diagnostics = app.state::<services::diagnostics::Diagnostics>();
            diagnostics.record("info", "application_started", None);
            if let Ok(local_data) = app.path().app_local_data_dir() {
                let report =
                    services::startup::cleanup_project_storage(&local_data.join("projects"));
                if report.partial_files_removed > 0 || report.cache_entries_removed > 0 {
                    diagnostics.record("info", "startup_cleanup_completed", None);
                }
                if report.failures > 0 {
                    diagnostics.record("warn", "startup_cleanup_incomplete", None);
                }
            }
            Ok(())
        })
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
            commands::diagnostics::select_diagnostic_destination,
            commands::diagnostics::create_diagnostic_bundle,
            commands::diagnostics::reveal_in_folder,
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
        .build(tauri::generate_context!())
        .expect("error while building Skull’d Clip Forge");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            use std::{thread, time::Duration};
            use tauri::Manager;

            let registry = app
                .state::<services::export::ExportRegistry>()
                .inner()
                .clone();
            if registry.is_active() {
                api.prevent_exit();
                registry.cancel_active();
                app.state::<services::diagnostics::Diagnostics>().record(
                    "info",
                    "shutdown_export_cancellation_requested",
                    None,
                );
                let app = app.clone();
                let _ = thread::Builder::new()
                    .name("skcf-shutdown-wait".to_owned())
                    .spawn(move || {
                        while registry.is_active() {
                            thread::sleep(Duration::from_millis(50));
                        }
                        app.exit(0);
                    });
            }
        }
    });
}
