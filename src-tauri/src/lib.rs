mod archive;

use archive::{ArchiveSummary, CreateOptions};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

#[cfg(windows)]
fn set_reg_value(path: &str, name: &str, data: &str) -> std::io::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(path)?;
    if name.is_empty() {
        key.set_value("", &data)
    } else {
        key.set_value(name, &data)
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
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_str = exe.to_string_lossy().to_string();
        let exe_quoted = format!("\"{exe_str}\"");

        // Idempotency check: skip the ~40 registry writes below entirely if
        // they already point at this exact exe path (e.g. every app launch,
        // not just the first). This — plus writing directly via the Win32
        // registry API instead of spawning `reg.exe` per value, which was
        // both flashing a console window each time (no window station to
        // attach a new console to without CREATE_NO_WINDOW) and adding real
        // process-spawn overhead 40+ times over — is what was causing the
        // freeze and the console flicker on every startup.
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(existing) = hkcu
            .open_subkey("Software\\Classes\\*\\shell\\Tugur\\shell\\01_add\\command")
            .and_then(|k| k.get_value::<String, _>(""))
        {
            if existing == format!("{exe_quoted} --add %*") {
                return Ok(true);
            }
        }

        for root in ["Software\\Classes\\*", "Software\\Classes\\Directory"] {
            let menu = format!("{root}\\shell\\Tugur");
            set_reg_value(&menu, "MUIVerb", "Tugur").map_err(|e| e.to_string())?;
            set_reg_value(&menu, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
            set_reg_value(&menu, "MultiSelectModel", "Player").map_err(|e| e.to_string())?;
            set_reg_value(&menu, "SubCommands", "").map_err(|e| e.to_string())?;

            let add = format!("{menu}\\shell\\01_add");
            set_reg_value(&add, "", "Add to archive...").map_err(|e| e.to_string())?;
            set_reg_value(&format!("{add}\\command"), "", &format!("{exe_quoted} --add %*"))
                .map_err(|e| e.to_string())?;

            let quick = format!("{menu}\\shell\\02_add_default");
            set_reg_value(&quick, "", "Add to .arc archive").map_err(|e| e.to_string())?;
            set_reg_value(
                &format!("{quick}\\command"),
                "",
                &format!("{exe_quoted} --add-default %*"),
            )
            .map_err(|e| e.to_string())?;

            let mail = format!("{menu}\\shell\\03_add_mail");
            set_reg_value(&mail, "", "Compress and email...").map_err(|e| e.to_string())?;
            set_reg_value(
                &format!("{mail}\\command"),
                "",
                &format!("{exe_quoted} --add-mail %*"),
            )
            .map_err(|e| e.to_string())?;
        }

        let background = "Software\\Classes\\Directory\\Background\\shell\\Tugur";
        set_reg_value(background, "MUIVerb", "Tugur").map_err(|e| e.to_string())?;
        set_reg_value(background, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
        set_reg_value(
            &format!("{background}\\command"),
            "",
            &format!("{exe_quoted} --add-default \"%V\""),
        )
        .map_err(|e| e.to_string())?;

        for extension in ["arc", "zip", "7z"] {
            let base = format!("Software\\Classes\\SystemFileAssociations\\.{extension}\\shell");
            let extract = format!("{base}\\TugurExtractHere");
            set_reg_value(&extract, "", "Extract Here").map_err(|e| e.to_string())?;
            set_reg_value(
                &format!("{extract}\\command"),
                "",
                &format!("{exe_quoted} --extract-here \"%1\""),
            )
            .map_err(|e| e.to_string())?;

            let open = format!("{base}\\TugurOpen");
            set_reg_value(&open, "", "Open with Tugur").map_err(|e| e.to_string())?;
            set_reg_value(&format!("{open}\\command"), "", &format!("{exe_quoted} \"%1\""))
                .map_err(|e| e.to_string())?;
        }

        let arc_type = "Software\\Classes\\.arc";
        set_reg_value(arc_type, "", "Tugur.Archive").map_err(|e| e.to_string())?;
        set_reg_value(arc_type, "Content Type", "application/x-tugur").map_err(|e| e.to_string())?;
        set_reg_value("Software\\Classes\\Tugur.Archive", "", "Tugur Archive").map_err(|e| e.to_string())?;
        set_reg_value(
            "Software\\Classes\\Tugur.Archive\\DefaultIcon",
            "",
            &format!("{exe_str},0"),
        )
        .map_err(|e| e.to_string())?;
        set_reg_value(
            "Software\\Classes\\Tugur.Archive\\shell\\open\\command",
            "",
            &format!("{exe_quoted} \"%1\""),
        )
        .map_err(|e| e.to_string())?;

        Ok(true)
    }
}

/// Opens `path` in the OS file manager. Deliberately not using
/// `tauri-plugin-shell`'s `open()` here: on Windows that shells through
/// `cmd /C start`, which flashes a console window for a frame even though
/// it exits instantly — the other source of the "cmd sometimes opening"
/// symptom (the registry registration above was the frequent one).
/// `explorer.exe` spawned directly with `CREATE_NO_WINDOW` has no console
/// to flash in the first place.
#[tauri::command]
fn reveal_in_file_manager(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("explorer.exe")
            .arg(&path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&path).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(&path).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
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

/// What Tugur was launched to do, decoded from argv. Populated by the Windows
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
            reveal_in_file_manager,
            list_archive,
            create_archive,
            delete_sources,
            extract_archive,
            test_archive,
            detect_format,
            get_launch_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tugur");
}
