mod archive;

use archive::{ArchiveSummary, CreateOptions};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// The .init file-type icon (the "wrapped box" logo — distinct from the
/// app's own "eye" icon), embedded directly in the binary so this works
/// regardless of Tauri's resource-bundling config, which doesn't have a
/// documented per-extension icon field for desktop file associations to
/// hook into. Written out next to the exe once so the registry's
/// DefaultIcon can point at a real file distinct from the app's own icon.
#[cfg(windows)]
const INIT_FILE_ICON: &[u8] = include_bytes!("../icons/filetype/init-file.ico");

#[cfg(windows)]
fn write_init_file_icon(exe: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));
    let ico_path = dir.join("init-file.ico");
    if !ico_path.exists() {
        std::fs::write(&ico_path, INIT_FILE_ICON)?;
    }
    Ok(ico_path)
}


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

/// Deletes whatever shell-menu/registry trees earlier Syncinit builds left
/// behind under the app's previous names ("Zarc", then "Tugur") before it
/// settled on "Syncinit". Nothing in this codebase writes those names
/// anymore, but an install that's been upgraded across the renames can
/// still have them sitting in the registry — showing up as duplicate/stale
/// "Zarc" or "Tugur" entries in the right-click menu (the "still named
/// zarc" symptom) instead of a clean single "Syncinit" entry. Best-effort:
/// every deletion is allowed to fail silently (key may simply not exist).
#[cfg(windows)]
fn cleanup_legacy_registrations() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for legacy in ["Zarc", "Tugur"] {
        for root in ["Software\\Classes\\*", "Software\\Classes\\Directory"] {
            let _ = hkcu.delete_subkey_all(&format!("{root}\\shell\\{legacy}"));
        }
        let _ = hkcu.delete_subkey_all(&format!(
            "Software\\Classes\\Directory\\Background\\shell\\{legacy}"
        ));
        for extension in ["init", "zip", "7z"] {
            let base = format!("Software\\Classes\\SystemFileAssociations\\.{extension}\\shell");
            let _ = hkcu.delete_subkey_all(&format!("{base}\\{legacy}ExtractHere"));
            let _ = hkcu.delete_subkey_all(&format!("{base}\\{legacy}Open"));
        }
        let _ = hkcu.delete_subkey_all(&format!("Software\\Classes\\{legacy}.Archive"));
    }
    // The old ".init" ProgID pointed at "Zarc.Archive"/"Tugur.Archive" before
    // this rename; if HKCU\...\.init\ is still set to one of those (rather
    // than "Syncinit.Archive"), Explorer's icon/open-with for .init files
    // can still resolve through the dead ProgID. register_context_menu()
    // rewrites it to "Syncinit.Archive" right after this call regardless.
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

        cleanup_legacy_registrations();

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
            .open_subkey("Software\\Classes\\*\\shell\\Syncinit\\shell\\01_add\\command")
            .and_then(|k| k.get_value::<String, _>(""))
        {
            if existing == format!("{exe_quoted} --add %*") {
                return Ok(true);
            }
        }

        for root in ["Software\\Classes\\*", "Software\\Classes\\Directory"] {
            let menu = format!("{root}\\shell\\Syncinit");
            set_reg_value(&menu, "MUIVerb", "Syncinit").map_err(|e| e.to_string())?;
            set_reg_value(&menu, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
            set_reg_value(&menu, "MultiSelectModel", "Player").map_err(|e| e.to_string())?;
            set_reg_value(&menu, "SubCommands", "").map_err(|e| e.to_string())?;

            // Only meaningful for actual files (not folders): "Open" and
            // "Extract Here" inside the SAME Syncinit submenu, restricted to
            // archive extensions via AppliesTo so they don't show up when
            // right-clicking a .txt file. This is what WinRAR's own menu
            // does — Open/Extract live in the same cascade as Add, not a
            // separate flat entry — which is what was actually missing
            // before, not a registration failure.
            if root.ends_with('*') {
                let open = format!("{menu}\\shell\\00_open");
                set_reg_value(&open, "", "Open").map_err(|e| e.to_string())?;
                set_reg_value(
                    &open,
                    "AppliesTo",
                    "System.FileExtension:=\".init\" OR System.FileExtension:=\".zip\" OR System.FileExtension:=\".7z\" OR System.FileExtension:=\".tar\" OR System.FileExtension:=\".gz\" OR System.FileExtension:=\".xz\" OR System.FileExtension:=\".zst\" OR System.FileExtension:=\".bz2\"",
                )
                .map_err(|e| e.to_string())?;
                set_reg_value(&format!("{open}\\command"), "", &format!("{exe_quoted} \"%1\""))
                    .map_err(|e| e.to_string())?;

                let extract = format!("{menu}\\shell\\04_extract");
                set_reg_value(&extract, "", "Extract Here").map_err(|e| e.to_string())?;
                set_reg_value(
                    &extract,
                    "AppliesTo",
                    "System.FileExtension:=\".init\" OR System.FileExtension:=\".zip\" OR System.FileExtension:=\".7z\" OR System.FileExtension:=\".tar\" OR System.FileExtension:=\".gz\" OR System.FileExtension:=\".xz\" OR System.FileExtension:=\".zst\" OR System.FileExtension:=\".bz2\"",
                )
                .map_err(|e| e.to_string())?;
                set_reg_value(
                    &format!("{extract}\\command"),
                    "",
                    &format!("{exe_quoted} --extract-here \"%1\""),
                )
                .map_err(|e| e.to_string())?;
            }

            let add = format!("{menu}\\shell\\01_add");
            set_reg_value(&add, "", "Add to archive...").map_err(|e| e.to_string())?;
            set_reg_value(&format!("{add}\\command"), "", &format!("{exe_quoted} --add %*"))
                .map_err(|e| e.to_string())?;

            let quick = format!("{menu}\\shell\\02_add_default");
            set_reg_value(&quick, "", "Add to .init archive").map_err(|e| e.to_string())?;
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

        let background = "Software\\Classes\\Directory\\Background\\shell\\Syncinit";
        set_reg_value(background, "MUIVerb", "Syncinit").map_err(|e| e.to_string())?;
        set_reg_value(background, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
        set_reg_value(
            &format!("{background}\\command"),
            "",
            &format!("{exe_quoted} --add-default \"%V\""),
        )
        .map_err(|e| e.to_string())?;

        for extension in ["init", "zip", "7z"] {
            let base = format!("Software\\Classes\\SystemFileAssociations\\.{extension}\\shell");
            let extract = format!("{base}\\SyncinitExtractHere");
            set_reg_value(&extract, "", "Extract Here").map_err(|e| e.to_string())?;
            set_reg_value(
                &format!("{extract}\\command"),
                "",
                &format!("{exe_quoted} --extract-here \"%1\""),
            )
            .map_err(|e| e.to_string())?;

            let open = format!("{base}\\SyncinitOpen");
            set_reg_value(&open, "", "Open with Syncinit").map_err(|e| e.to_string())?;
            set_reg_value(&format!("{open}\\command"), "", &format!("{exe_quoted} \"%1\""))
                .map_err(|e| e.to_string())?;
        }

        let init_type = "Software\\Classes\\.init";
        let init_icon_path = write_init_file_icon(&exe)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| exe_str.clone()); // fall back to the app icon if the write fails
        set_reg_value(init_type, "", "Syncinit.Archive").map_err(|e| e.to_string())?;
        set_reg_value(init_type, "Content Type", "application/x-syncinit").map_err(|e| e.to_string())?;
        set_reg_value("Software\\Classes\\Syncinit.Archive", "", "Syncinit Archive").map_err(|e| e.to_string())?;
        set_reg_value(
            "Software\\Classes\\Syncinit.Archive\\DefaultIcon",
            "",
            &format!("{init_icon_path},0"),
        )
        .map_err(|e| e.to_string())?;
        set_reg_value(
            "Software\\Classes\\Syncinit.Archive\\shell\\open\\command",
            "",
            &format!("{exe_quoted} \"%1\""),
        )
        .map_err(|e| e.to_string())?;

        Ok(true)
    }
}

/// Reads back what's actually in the registry right now, so a registration
/// problem can be seen instead of guessed at. Call from the devtools
/// console: `await window.__TAURI__.core.invoke("debug_context_menu")`,
/// or wire a temporary button to it. Returns the exe path we registered
/// (compare it against the exe you're actually running — a stale build
/// pointing at an old path is the most common reason "it doesn't show up"
/// after a fix that looks correct on the Rust side), plus whether the key
/// Explorer actually needs to render Syncinit as a cascading menu is present.
#[tauri::command]
fn debug_context_menu() -> Result<serde_json::Value, String> {
    #[cfg(not(windows))]
    {
        return Ok(serde_json::json!({ "platform": "not windows, no registry to check" }));
    }
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        let read_str = |path: &str, name: &str| -> Option<String> {
            hkcu.open_subkey(path).ok()?.get_value::<String, _>(name).ok()
        };

        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().to_string();
        let file_menu_key = "Software\\Classes\\*\\shell\\Syncinit";
        let dir_menu_key = "Software\\Classes\\Directory\\shell\\Syncinit";

        Ok(serde_json::json!({
            "running_exe": current_exe,
            "file_menu": {
                "key_exists": hkcu.open_subkey(file_menu_key).is_ok(),
                "MUIVerb": read_str(file_menu_key, "MUIVerb"),
                "SubCommands_present": read_str(file_menu_key, "SubCommands").is_some(),
                "add_command": read_str(&format!("{file_menu_key}\\shell\\01_add\\command"), ""),
                "open_command": read_str(&format!("{file_menu_key}\\shell\\00_open\\command"), ""),
            },
            "directory_menu": {
                "key_exists": hkcu.open_subkey(dir_menu_key).is_ok(),
                "MUIVerb": read_str(dir_menu_key, "MUIVerb"),
            },
            "note": "If key_exists is false, registration never ran or failed silently — check the console.warn from register_context_menu. If key_exists is true but the menu still doesn't show: (1) confirm running_exe above matches the .exe you're actually launching, not a stale build; (2) on Windows 11, non-pinned right-click entries land under \"Show more options\" at the bottom of the compact menu, not the top-level list.",
        }))
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

/// Tracks the cancel flag for whichever create/extract is currently
/// running. Syncinit only ever runs one at a time (single window, one
/// modal-driven flow), so a single slot is enough.
struct CancelState(Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>>);

#[derive(Clone, Serialize)]
struct ProgressPayload {
    done: u64,
    total: u64,
}

fn begin_operation(state: &tauri::State<CancelState>) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    *state.0.lock().unwrap() = Some(flag.clone());
    flag
}

#[tauri::command]
fn cancel_operation(state: tauri::State<CancelState>) {
    if let Some(flag) = state.0.lock().unwrap().as_ref() {
        flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

#[tauri::command]
fn list_archive(path: String, password: Option<String>) -> Result<ArchiveSummary, String> {
    archive::list_archive(&path, password.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_archive(
    app: tauri::AppHandle,
    state: tauri::State<CancelState>,
    options: CreateOptions,
) -> Result<(), String> {
    let cancel_flag = begin_operation(&state);
    // Throttled: emitting on every 64 KiB chunk of a multi-GB file floods
    // the webview IPC channel and is what was making the UI stutter/freeze
    // during large operations elsewhere in this app — cap it to ~20/sec.
    let last_emit = std::cell::Cell::new(std::time::Instant::now());
    let emit_app = app.clone();
    let on_progress = move |done: u64, total: u64| {
        if last_emit.get().elapsed().as_millis() < 50 && done < total {
            return;
        }
        last_emit.set(std::time::Instant::now());
        let _ = emit_app.emit("syncinit://progress", ProgressPayload { done, total });
    };
    let ctx = archive::ProgressCtx { on_progress: Some(&on_progress), cancelled: Some(&cancel_flag) };
    archive::create_archive(&options, &ctx).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_sources(sources: Vec<String>, destination: String) -> Result<usize, String> {
    archive::delete_sources(&sources, &destination).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_entries(path: String, names: Vec<String>) -> Result<usize, String> {
    archive::delete_entries(&path, &names, &archive::ProgressCtx::none()).map_err(|e| e.to_string())
}

#[tauri::command]
fn extract_archive(
    app: tauri::AppHandle,
    state: tauri::State<CancelState>,
    path: String,
    destination: String,
    password: Option<String>,
) -> Result<usize, String> {
    let cancel_flag = begin_operation(&state);
    let last_emit = std::cell::Cell::new(std::time::Instant::now());
    let emit_app = app.clone();
    let on_progress = move |done: u64, total: u64| {
        if last_emit.get().elapsed().as_millis() < 50 && done < total {
            return;
        }
        last_emit.set(std::time::Instant::now());
        let _ = emit_app.emit("syncinit://progress", ProgressPayload { done, total });
    };
    let ctx = archive::ProgressCtx { on_progress: Some(&on_progress), cancelled: Some(&cancel_flag) };
    archive::extract_archive(&path, &destination, password.as_deref(), &ctx).map_err(|e| e.to_string())
}

#[tauri::command]
fn test_archive(path: String) -> Result<bool, String> {
    archive::test_archive(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn detect_format(path: String) -> Option<String> {
    archive::Format::from_path(std::path::Path::new(&path)).map(|f| format!("{:?}", f))
}

/// What Syncinit was launched to do, decoded from argv. Populated by the Windows
/// Explorer context-menu entries registered at install time (see
/// src-tauri/installer-hooks.nsh) — mirrors WinRAR's Add to archive... /
/// Add to "name.init" / Compress and email... items.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
enum LaunchAction {
    /// "Add to archive..." — open the Archive options dialog prefilled with these paths.
    AddDialog {
        paths: Vec<String>,
    },
    /// "Add to <name>.init" — build the archive immediately next to the source, no dialog.
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
        // Must be the first plugin registered. When a second copy of
        // Syncinit is launched (e.g. Explorer's "Add to archive…" while the
        // app is already open), this hands the new instance's argv to the
        // *existing* process via this callback and lets the new process
        // exit — instead of a second full window opening (or racing the
        // first one over the same files), which is what "Add to archive…
        // doesn't work" looked like whenever the app was already running.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let action = parse_launch_action(&argv);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            if !matches!(action, LaunchAction::None) {
                let _ = app.emit("syncinit://relaunch-action", action);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(LaunchState(Mutex::new(launch_action)))
        .manage(CancelState(Mutex::new(None)))
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
            debug_context_menu,
            cancel_operation,
            reveal_in_file_manager,
            list_archive,
            create_archive,
            delete_sources,
            delete_entries,
            extract_archive,
            test_archive,
            detect_format,
            get_launch_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running Syncinit");
}
