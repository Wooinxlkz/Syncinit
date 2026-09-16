mod archive;

use archive::{ArchiveSummary, CreateOptions};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

#[cfg(windows)]
fn reg_add(key: &str, value: Option<&str>, data: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("reg.exe");
    command.args(["add", key]);
    if let Some(value) = value {
        command.args(["/v", value]);
    } else {
        command.arg("/ve");
    }
    command.args(["/t", "REG_SZ", "/d", data, "/f"]);
    let status = command.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("reg.exe failed for {key}"))
    }
}

#[tauri::command]
fn register_context_menu() -> Result<bool, String> {
    #[cfg(not(windows))]
    {
        return Ok(false);
    }

    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe = format!("\"{}\"", exe.to_string_lossy());

        for root in [
            "HKCU\\Software\\Classes\\*",
            "HKCU\\Software\\Classes\\Directory",
        ] {
            let menu = format!("{root}\\shell\\Zarc");
            reg_add(&menu, Some("MUIVerb"), "Zarc")?;
            reg_add(&menu, Some("Icon"), &format!("{exe},0"))?;
            reg_add(&menu, Some("MultiSelectModel"), "Player")?;

            let add = format!("{menu}\\shell\\01_add");
            reg_add(&add, None, "Add to archive...")?;
            reg_add(&format!("{add}\\command"), None, &format!("{exe} --add %*"))?;

            let quick = format!("{menu}\\shell\\02_add_default");
            reg_add(&quick, None, "Add to .arc archive")?;
            reg_add(
                &format!("{quick}\\command"),
                None,
                &format!("{exe} --add-default %*"),
            )?;

            let mail = format!("{menu}\\shell\\03_add_mail");
            reg_add(&mail, None, "Compress and email...")?;
            reg_add(
                &format!("{mail}\\command"),
                None,
                &format!("{exe} --add-mail %*"),
            )?;
        }

        let background = "HKCU\\Software\\Classes\\Directory\\Background\\shell\\Zarc";
        reg_add(background, Some("MUIVerb"), "Zarc")?;
        reg_add(background, Some("Icon"), &format!("{exe},0"))?;
        reg_add(
            &format!("{background}\\command"),
            None,
            &format!("{exe} --add-default \"%V\""),
        )?;

        for extension in ["arc", "zip", "7z"] {
            let base =
                format!("HKCU\\Software\\Classes\\SystemFileAssociations\\.{extension}\\shell");
            let extract = format!("{base}\\ZarcExtractHere");
            reg_add(&extract, None, "Extract Here")?;
            reg_add(
                &format!("{extract}\\command"),
                None,
                &format!("{exe} --extract-here \"%1\""),
            )?;

            let open = format!("{base}\\ZarcOpen");
            reg_add(&open, None, "Open with Zarc")?;
            reg_add(&format!("{open}\\command"), None, &format!("{exe} \"%1\""))?;
        }

        let arc_type = "HKCU\\Software\\Classes\\.arc";
        reg_add(arc_type, None, "Zarc.Archive")?;
        reg_add(arc_type, Some("Content Type"), "application/x-zarc")?;
        reg_add(
            "HKCU\\Software\\Classes\\Zarc.Archive",
            None,
            "Zarc Archive",
        )?;
        reg_add(
            "HKCU\\Software\\Classes\\Zarc.Archive\\DefaultIcon",
            None,
            &format!("{exe},0"),
        )?;
        reg_add(
            "HKCU\\Software\\Classes\\Zarc.Archive\\shell\\open\\command",
            None,
            &format!("{exe} \"%1\""),
        )?;

        Ok(true)
    }
}

#[tauri::command]
fn list_archive(path: String, password: Option<String>) -> Result<ArchiveSummary, String> {
    archive::list_archive(&path, password.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_archive(options: CreateOptions) -> Result<(), String> {
    archive::create_archive(&options).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_sources(sources: Vec<String>, destination: String) -> Result<usize, String> {
    archive::delete_sources(&sources, &destination).map_err(|e| e.to_string())
}

#[tauri::command]
fn extract_archive(
    path: String,
    destination: String,
    password: Option<String>,
) -> Result<usize, String> {
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

/// What Zarc was launched to do, decoded from argv. Populated by the Windows
/// Explorer context-menu entries registered at install time (see
/// src-tauri/installer-hooks.nsh) — mirrors WinRAR's Add to archive... /
/// Add to "name.arc" / Compress and email... items.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
enum LaunchAction {
    /// "Add to archive..." — open the Archive options dialog prefilled with these paths.
    AddDialog {
        paths: Vec<String>,
    },
    /// "Add to <name>.arc" — build the archive immediately next to the source, no dialog.
    AddDefault {
        paths: Vec<String>,
    },
    /// "Compress and email..." — archive, then reveal the result so it can be attached.
    AddAndMail {
        paths: Vec<String>,
    },
    /// "Extract Here" from the archive's own context menu.
    ExtractHere {
        paths: Vec<String>,
    },
    /// Plain launch, e.g. double-clicking an archive file or opening the app directly.
    OpenArchive {
        path: String,
    },
    None,
}

struct LaunchState(Mutex<LaunchAction>);

fn parse_launch_action(args: &[String]) -> LaunchAction {
    // args[0] is the exe path.
    if args.len() < 2 {
        return LaunchAction::None;
    }
    let flag = args[1].as_str();
    let rest: Vec<String> = args[2..].to_vec();

    match flag {
        "--add" if !rest.is_empty() => LaunchAction::AddDialog { paths: rest },
        "--add-default" if !rest.is_empty() => LaunchAction::AddDefault { paths: rest },
        "--add-mail" if !rest.is_empty() => LaunchAction::AddAndMail { paths: rest },
        "--extract-here" if !rest.is_empty() => LaunchAction::ExtractHere { paths: rest },
        other if archive::Format::from_path(std::path::Path::new(other)).is_some() => {
            LaunchAction::OpenArchive {
                path: other.to_string(),
            }
        }
        _ => LaunchAction::None,
    }
}

#[tauri::command]
fn get_launch_action(state: tauri::State<LaunchState>) -> LaunchAction {
    let mut guard = state.0.lock().unwrap();
    let action = guard.clone();
    // Only deliver it once per process launch so re-focusing the window later
    // doesn't replay the same "add these files" action.
    *guard = LaunchAction::None;
    action
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch_action = parse_launch_action(&std::env::args().collect::<Vec<_>>());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(LaunchState(Mutex::new(launch_action)))
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            register_context_menu,
            list_archive,
            create_archive,
            delete_sources,
            extract_archive,
            test_archive,
            detect_format,
            get_launch_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zarc");
}
