mod api;
mod commands;
mod models;
mod mpv;
mod platform_window;
mod playback_watch;
mod store;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    platform_window::prepare_linux_wayland_environment()
        .expect("Zplayer Linux desktop requires a Wayland session.");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            platform_window::create_main_window(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { .. } => {
                mpv::terminate_all();
            }
            tauri::WindowEvent::Resized(_)
            | tauri::WindowEvent::ScaleFactorChanged { .. }
            | tauri::WindowEvent::Focused(true) => {
                mpv::restack_all(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_server_login,
            commands::save_server,
            commands::update_server_icon,
            commands::list_servers,
            commands::export_servers,
            commands::import_servers,
            commands::load_settings,
            commands::save_settings,
            commands::set_active_server,
            commands::delete_server,
            commands::load_home,
            commands::load_home_more,
            commands::load_watch_calendar,
            commands::load_library,
            commands::load_item,
            commands::load_item_more,
            commands::load_media_sources,
            commands::search_items,
            commands::play_item,
            commands::control_playback,
            commands::playback_state,
            commands::save_playback_preference,
            commands::load_playback_preferences,
            commands::mark_favorite,
            commands::mark_played,
            commands::fetch_server_name,
            commands::linux_window_diagnostics,
            commands::report_playback_start,
            commands::report_playback_progress,
            commands::report_playback_stopped
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
