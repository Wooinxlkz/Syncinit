mod archive;

use archive::{ArchiveSummary, CreateOptions};
use tauri::Manager;

#[tauri::command]
fn list_archive(path: String) -> Result<ArchiveSummary, String> {
    archive::list_archive(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_archive(options: CreateOptions) -> Result<(), String> {
    archive::create_archive(&options).map_err(|e| e.to_string())
}

#[tauri::command]
fn extract_archive(path: String, destination: String, password: Option<String>) -> Result<usize, String> {
    archive::extract_archive(&path, &destination, password.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn test_archive(path: String) -> Result<bool, String> {
    archive::test_archive(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn detect_format(path: String) -> Option<String> {
    archive::Format::from_path(std::path::Path::new(&path)).map(|f| format!("{:?}", f))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_archive,
            create_archive,
            extract_archive,
            test_archive,
            detect_format
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zarc");
}
