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
fn write_init_file_icon() -> std::io::Result<std::path::PathBuf> {
    // Deliberately NOT next to the exe anymore. A normal (non-admin) install
    // lives under `C:\Program Files\Syncinit\`, which a standard user can't
    // write to — so writing the icon there silently failed, and the code
    // fell back to using the *exe itself* as the DefaultIcon target, which
    // is the main app icon. That's the actual reason .init files kept
    // showing the main icon instead of the file-type one: it was the
    // fallback, not a caching issue. %LOCALAPPDATA% is always writable by
    // the current user regardless of where the app is installed.
    let base = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("Syncinit");
    std::fs::create_dir_all(&dir)?;
    let ico_path = dir.join("init-file.ico");
    // Always overwrite with whatever's embedded in *this* build. The old
    // `if !ico_path.exists()` guard here meant an upgrade with the same
    // install path never replaced an icon written by an earlier version —
    // exactly the "new logo isn't showing up" symptom, since the file on
    // disk (and thus the registry's DefaultIcon target) was frozen at
    // whatever the very first install wrote. A plain overwrite is cheap
    // (a few KB) and Explorer picks up the new bytes at the same path fine.
    std::fs::write(&ico_path, INIT_FILE_ICON)?;
    Ok(ico_path)
}

/// On-demand check of the .init icon's actual on-disk state — for
/// Settings' Diagnostics page, so "is the icon fix actually working" is a
/// direct answer instead of a guess: does the file exist at
/// %LOCALAPPDATA%\Syncinit\init-file.ico, how big is it, when was it last
/// written. If it's missing or 0 bytes, the write is failing (permissions,
/// disk space); if it's there and recent but Explorer still shows the
/// wrong icon, that points at Explorer's own icon cache instead.
#[tauri::command]
fn get_icon_diagnostics() -> serde_json::Value {
    #[cfg(not(windows))]
    {
        serde_json::json!({ "platform": "non-windows, not applicable" })
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let ico_path = base.join("Syncinit").join("init-file.ico");
        match std::fs::metadata(&ico_path) {
            Ok(meta) => {
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs());
                serde_json::json!({
                    "path": ico_path.to_string_lossy(),
                    "exists": true,
                    "size_bytes": meta.len(),
                    "modified_unix": modified,
                    "embedded_size_bytes": INIT_FILE_ICON.len(),
                })
            }
            Err(e) => serde_json::json!({
                "path": ico_path.to_string_lossy(),
                "exists": false,
                "error": e.to_string(),
            }),
        }
    }
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

/// The inverse of registration — removes every registry entry Syncinit
/// itself writes (the "Syncinit" shell verb tree under `*`/`Directory`/
/// `Directory\Background`, the `.init` file association, and the
/// `Syncinit.Archive` ProgID). Paired with `register_context_menu` /
/// `reinstall_context_menu` as an explicit, user-triggered install /
/// status / uninstall set — matching the pattern open-source archivers
/// like Ziplark ship (`shell-integration install/status/uninstall`)
/// instead of silent, invisible auto-registration on every launch with no
/// way to actually undo it short of hand-editing the registry.
#[tauri::command]
async fn uninstall_context_menu() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(windows)]
        {
            use winreg::enums::HKEY_CURRENT_USER;
            use winreg::RegKey;
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            for root in ["Software\\Classes\\*", "Software\\Classes\\Directory"] {
                let _ = hkcu.delete_subkey_all(&format!("{root}\\shell\\Syncinit"));
            }
            let _ = hkcu.delete_subkey_all("Software\\Classes\\Directory\\Background\\shell\\Syncinit");
            let _ = hkcu.delete_subkey_all("Software\\Classes\\.init");
            let _ = hkcu.delete_subkey_all("Software\\Classes\\Syncinit.Archive");
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Re-runs registration unconditionally, bypassing the up-to-date check
/// register_context_menu normally short-circuits on — for the explicit
/// "Reinstall" button in Diagnostics, so a person can force a clean
/// re-registration on demand rather than needing a version bump to make
/// it actually run.
#[tauri::command]
async fn reinstall_context_menu(app: tauri::AppHandle) -> Result<bool, String> {
    let result = tauri::async_runtime::spawn_blocking(|| register_context_menu_impl(true))
        .await
        .map_err(|e| e.to_string())?;
    log_diag(&app.state::<DiagnosticsLog>(), format!("reinstall_context_menu result: {result:?}"));
    result
}

#[tauri::command]
async fn register_context_menu(app: tauri::AppHandle) -> Result<bool, String> {
    // ~40 registry writes on a fresh install/upgrade — fast (a handful of
    // ms) but still main-thread work every single launch if left as a sync
    // command; same fix as the heavy archive commands above.
    let result = tauri::async_runtime::spawn_blocking(|| register_context_menu_impl(false))
        .await
        .map_err(|e| e.to_string())?;
    log_diag(&app.state::<DiagnosticsLog>(), format!("register_context_menu result: {result:?}"));
    result
}

fn register_context_menu_impl(force: bool) -> Result<bool, String> {
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
        // they already point at this exact exe path *and* were written by
        // this same build. Gating on version (not just the exe path) matters
        // because this function also rewrites the .init file icon and the
        // menu's Icon/MUIVerb values — on an upgrade that installs to the
        // same path, the old path-only check saw an identical command string
        // and returned early forever, so a new release's icon/menu changes
        // never actually reached the registry. This — plus writing directly
        // via the Win32 registry API instead of spawning `reg.exe` per
        // value, which was both flashing a console window each time (no
        // window station to attach a new console to without
        // CREATE_NO_WINDOW) and adding real process-spawn overhead 40+
        // times over — is what was causing the freeze and the console
        // flicker on every startup. `force` (the explicit "Reinstall"
        // button in Diagnostics) bypasses this entirely.
        const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let up_to_date = !force
            && hkcu
                .open_subkey("Software\\Classes\\*\\shell\\Syncinit\\shell\\01_add\\command")
                .and_then(|k| k.get_value::<String, _>(""))
                .map(|existing| existing == format!("{exe_quoted} --add %1"))
                .unwrap_or(false)
            && hkcu
                .open_subkey("Software\\Classes\\*\\shell\\Syncinit")
                .and_then(|k| k.get_value::<String, _>("SyncinitVersion"))
                .map(|v| v == APP_VERSION)
                .unwrap_or(false);
        if up_to_date {
            return Ok(true);
        }

        for root in ["Software\\Classes\\*", "Software\\Classes\\Directory"] {
            let menu = format!("{root}\\shell\\Syncinit");
            set_reg_value(&menu, "MUIVerb", "Syncinit").map_err(|e| e.to_string())?;
            set_reg_value(&menu, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
            set_reg_value(&menu, "MultiSelectModel", "Player").map_err(|e| e.to_string())?;
            set_reg_value(&menu, "SubCommands", "").map_err(|e| e.to_string())?;
            // Marker this idempotency check reads back above — only ever
            // written on the "*" root's key (the one the check reads), but
            // harmless to set on both since it's the same literal value.
            set_reg_value(&menu, "SyncinitVersion", APP_VERSION).map_err(|e| e.to_string())?;

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
                set_reg_value(&open, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
                set_reg_value(
                    &open,
                    "AppliesTo",
                    "System.FileExtension:=\".init\" OR System.FileExtension:=\".zip\" OR System.FileExtension:=\".7z\" OR System.FileExtension:=\".tar\" OR System.FileExtension:=\".gz\" OR System.FileExtension:=\".xz\" OR System.FileExtension:=\".zst\" OR System.FileExtension:=\".bz2\" OR System.FileExtension:=\".rar\"",
                )
                .map_err(|e| e.to_string())?;
                set_reg_value(&format!("{open}\\command"), "", &format!("{exe_quoted} \"%1\""))
                    .map_err(|e| e.to_string())?;

                let extract = format!("{menu}\\shell\\04_extract");
                set_reg_value(&extract, "", "Extract Here").map_err(|e| e.to_string())?;
                set_reg_value(&extract, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
                set_reg_value(
                    &extract,
                    "AppliesTo",
                    "System.FileExtension:=\".init\" OR System.FileExtension:=\".zip\" OR System.FileExtension:=\".7z\" OR System.FileExtension:=\".tar\" OR System.FileExtension:=\".gz\" OR System.FileExtension:=\".xz\" OR System.FileExtension:=\".zst\" OR System.FileExtension:=\".bz2\" OR System.FileExtension:=\".rar\"",
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
            set_reg_value(&add, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
            // MultiSelectModel set on this verb (not just the cascading
            // parent) per Microsoft's docs — real cause of the *actual*
            // bug turned out to be simpler and more basic than that,
            // though: `%*` itself is well-documented as unreliable in
            // registry shell commands — community-collected evidence
            // ("%2 or %* are blank, even if multiple files are selected")
            // and a real diagnostics capture from this app both show it
            // expanding to nothing even for a single selected file. `%1`
            // is universally reliable and is what Open/Extract Here
            // already use below. Multi-file selection with Player set
            // still launches once per file in practice (not one call with
            // every path) — each launch reaches the single-instance
            // handler and is handled there rather than lost, which is why
            // MultiSelectModel is still worth keeping set.
            set_reg_value(&add, "MultiSelectModel", "Player").map_err(|e| e.to_string())?;
            set_reg_value(&format!("{add}\\command"), "", &format!("{exe_quoted} --add %1"))
                .map_err(|e| e.to_string())?;

            let quick = format!("{menu}\\shell\\02_add_default");
            set_reg_value(&quick, "", "Add to .init archive").map_err(|e| e.to_string())?;
            set_reg_value(&quick, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
            set_reg_value(&quick, "MultiSelectModel", "Player").map_err(|e| e.to_string())?;
            set_reg_value(
                &format!("{quick}\\command"),
                "",
                &format!("{exe_quoted} --add-default %1"),
            )
            .map_err(|e| e.to_string())?;

            let mail = format!("{menu}\\shell\\03_add_mail");
            set_reg_value(&mail, "", "Compress and email...").map_err(|e| e.to_string())?;
            set_reg_value(&mail, "Icon", &format!("{exe_str},0")).map_err(|e| e.to_string())?;
            set_reg_value(&mail, "MultiSelectModel", "Player").map_err(|e| e.to_string())?;
            set_reg_value(
                &format!("{mail}\\command"),
                "",
                &format!("{exe_quoted} --add-mail %1"),
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

        for extension in ["init", "zip", "7z", "rar"] {
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
        let init_icon_path = write_init_file_icon()
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

/// Opens a URL in the OS default browser — used for the update checker's
/// "View release" link. Uses the `open` crate rather than
/// @tauri-apps/plugin-shell's JS open() (which shells out via cmd.exe on
/// Windows and flashes a console window) or a hand-rolled `cmd /C start`
/// (which has a real, if currently low-risk, shell-metacharacter issue:
/// cmd.exe re-parses the command line a second time after Rust's own argv
/// quoting, so a URL containing &, |, ^, etc. could be misinterpreted).
/// `open` calls ShellExecuteW directly on Windows — no console window, no
/// second parsing pass, no cmd.exe involved at all.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Refusing to open a non-https URL".into());
    }
    open::that_detached(&url).map_err(|e| e.to_string())
}

/// Tracks the cancel flag for whichever create/extract operation is currently
/// running. Syncinit only ever runs one at a time (single window, one
/// modal-driven flow), so a single slot is enough.
struct CancelState(Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>>);

#[derive(Clone, Serialize)]
struct ProgressPayload {
    done: u64,
    total: u64,
}

#[derive(Serialize)]
struct ExtractResult {
    count: usize,
    // Paths that couldn't be extracted (corrupt entry, unsafe path, I/O
    // error) — the extraction still completes with everything that *was*
    // readable instead of aborting on the first bad entry (matches the
    // same "keep what's readable" behavior create already has).
    warnings: Vec<String>,
}

fn begin_operation(state: &tauri::State<'_, CancelState>) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
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

// Every command below that touches the filesystem for real (as opposed to
// the near-instant registry/path-string ones above) is `async fn` wrapping
// its actual work in `tauri::async_runtime::spawn_blocking`. This matters:
// in Tauri 2, a *synchronous* `fn` command runs on the main thread — the
// same thread that pumps the window's event loop and repaints the webview.
// These used to all be plain `fn`, so compressing/extracting/testing a real
// archive froze the entire window (toolbar included, not just the operation
// itself) for as long as it took. `spawn_blocking` moves the work onto a
// dedicated blocking-friendly thread from Tokio's pool and `.await`s it, so
// the main thread stays free the whole time.

#[tauri::command]
async fn list_archive(path: String, password: Option<String>) -> Result<ArchiveSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        archive::list_archive(&path, password.as_deref()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn create_archive(
    app: tauri::AppHandle,
    state: tauri::State<'_, CancelState>,
    options: CreateOptions,
) -> Result<Vec<String>, String> {
    let cancel_flag = begin_operation(&state);
    tauri::async_runtime::spawn_blocking(move || {
        // Throttled: emitting on every 64 KiB chunk of a multi-GB file floods
        // the webview IPC channel and is what was making the UI stutter/freeze
        // during large operations elsewhere in this app — cap it to ~20/sec.
        let last_emit = std::cell::Cell::new(std::time::Instant::now());
        let on_progress = move |done: u64, total: u64| {
            if last_emit.get().elapsed().as_millis() < 50 && done < total {
                return;
            }
            last_emit.set(std::time::Instant::now());
            let _ = app.emit("syncinit://progress", ProgressPayload { done, total });
        };
        let ctx = archive::ProgressCtx { on_progress: Some(&on_progress), cancelled: Some(&cancel_flag) };
        // Ok(warnings): a non-empty list means the archive was created but
        // some paths couldn't be added (see archive::create_archive's own
        // doc comment) — the frontend surfaces these rather than the old
        // behavior of silently pretending everything made it in.
        archive::create_archive(&options, &ctx).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn delete_sources(sources: Vec<String>, destination: String) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        archive::delete_sources(&sources, &destination).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn delete_entries(path: String, names: Vec<String>) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        archive::delete_entries(&path, &names, &archive::ProgressCtx::none()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Set, change, or remove a password/PIN on an already-created archive —
/// wasn't possible before (password was only ever set at creation time).
/// `new_password: None` (or empty string from the UI) removes protection.
#[tauri::command]
async fn change_archive_password(
    path: String,
    old_password: Option<String>,
    new_password: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        archive::change_password(
            &path,
            old_password.as_deref(),
            new_password.as_deref().filter(|p| !p.is_empty()),
            &archive::ProgressCtx::none(),
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

const KEYRING_SERVICE: &str = "Syncinit Archive Password";

/// Canonicalizing the path before using it as the credential's account
/// name means a password saved for one relative/symlinked route to a file
/// is still found via another. Falls back to the raw path if canonicalize
/// fails (a network path with permission quirks, say) rather than
/// blocking the save entirely — worst case the key is just a little more
/// fragile to a later rename, not broken.
fn keyring_entry_for(path: &str) -> Result<keyring::Entry, String> {
    let key = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string());
    keyring::Entry::new(KEYRING_SERVICE, &key).map_err(|e| e.to_string())
}

/// Saves a password/PIN to the OS credential store (Windows Credential
/// Manager) for this archive — opt-in only, via the "Remember this
/// password" checkbox on the unlock dialog. Syncinit itself never writes
/// the password to disk in plaintext; this hands it to Windows' own
/// secure store and only ever keeps the archive's path as a lookup key.
#[tauri::command]
async fn save_archive_password(path: String, password: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        keyring_entry_for(&path)?.set_password(&password).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Returns the saved password if one exists — `Ok(None)` (not an error)
/// when there's simply nothing saved for this archive yet, which is the
/// normal case for any archive the person hasn't opted into remembering.
#[tauri::command]
async fn get_saved_archive_password(path: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || match keyring_entry_for(&path)?.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Removes a saved password — called when the person unchecks "remember"
/// after previously saving one, when a saved password turns out to be
/// stale (archive's password changed elsewhere), or when password
/// protection is removed from the archive entirely. `NoEntry` isn't an
/// error here: "nothing to forget" is a successful no-op, not a failure.
#[tauri::command]
async fn forget_archive_password(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || match keyring_entry_for(&path)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extract_archive(
    app: tauri::AppHandle,
    state: tauri::State<'_, CancelState>,
    path: String,
    destination: String,
    password: Option<String>,
) -> Result<ExtractResult, String> {
    let cancel_flag = begin_operation(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let last_emit = std::cell::Cell::new(std::time::Instant::now());
        let on_progress = move |done: u64, total: u64| {
            if last_emit.get().elapsed().as_millis() < 50 && done < total {
                return;
            }
            last_emit.set(std::time::Instant::now());
            let _ = app.emit("syncinit://progress", ProgressPayload { done, total });
        };
        let ctx = archive::ProgressCtx { on_progress: Some(&on_progress), cancelled: Some(&cancel_flag) };
        archive::extract_archive(&path, &destination, password.as_deref(), &ctx)
            .map(|(count, warnings)| ExtractResult { count, warnings })
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn test_archive(path: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || archive::test_archive(&path).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
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

/// A running, timestamped log of the launch/registration lifecycle —
/// startup argv, what it parsed into, single-instance relaunches, context
/// menu registration outcomes, icon-write outcomes. Exists so "it's not
/// working" has somewhere to point: read back via `get_diagnostics` (shown
/// on Settings' Diagnostics page, with a one-click copy) instead of asking
/// someone to dig through devtools. Capped so it can't grow unbounded
/// across a long-running session.
struct DiagnosticsLog(Mutex<Vec<String>>);

fn log_diag(state: &tauri::State<DiagnosticsLog>, entry: impl Into<String>) {
    let mut log = state.0.lock().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    log.push(format!("[{now}] {}", entry.into()));
    if log.len() > 200 {
        let excess = log.len() - 200;
        log.drain(0..excess);
    }
}

#[tauri::command]
fn get_diagnostics(state: tauri::State<DiagnosticsLog>) -> Vec<String> {
    state.0.lock().unwrap().clone()
}

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

/// Raw process argv, logged to the devtools console on startup — a way to
/// actually see what Explorer passed for "Add to archive…" etc. instead of
/// guessing blind. If this doesn't show the expected `--add "C:\...\file"`
/// shape, the bug is in the registry command string / how Windows invoked
/// it; if it does and the dialog still didn't open, the bug is downstream
/// in the frontend's handling of it.
#[tauri::command]
fn get_raw_args() -> Vec<String> {
    std::env::args().collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let raw_argv: Vec<String> = std::env::args().collect();
    let launch_action = parse_launch_action(&raw_argv);
    let diagnostics = DiagnosticsLog(Mutex::new(vec![
        format!("startup argv: {raw_argv:?}"),
        format!("startup parsed as: {launch_action:?}"),
    ]));

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
            log_diag(&app.state::<DiagnosticsLog>(), format!("relaunch argv: {argv:?}"));
            log_diag(&app.state::<DiagnosticsLog>(), format!("relaunch parsed as: {action:?}"));
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
        .manage(diagnostics)
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
            reinstall_context_menu,
            uninstall_context_menu,
            debug_context_menu,
            cancel_operation,
            reveal_in_file_manager,
            open_url,
            list_archive,
            create_archive,
            delete_sources,
            delete_entries,
            change_archive_password,
            save_archive_password,
            get_saved_archive_password,
            forget_archive_password,
            extract_archive,
            test_archive,
            detect_format,
            get_launch_action,
            get_raw_args,
            get_diagnostics,
            get_icon_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running Syncinit");
}
