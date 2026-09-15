# Zarc v0.1.1

A fast, open, modern archive manager — Rust + Tauri v2 + React. Built to eventually
surpass WinRAR on speed, format support, UI, and security, while staying free/open.

## Stack
- Backend: Rust, Tauri v2, `zip`/`sevenz-rust`/`tar`/`flate2`/`xz2`/`zstd`/`bzip2`
- Frontend: React 18 + TypeScript, built with Vite, managed with bun
- Window: fixed-small default (900x600, resizable, WinRAR-style), single dark theme for now

## Run it
```bash
bun install
bun run tauri dev
```
Building a release bundle:
```bash
bun run tauri build
```

## What's actually working in v0.1.0
- List contents of `.zip`, `.tar`, `.tar.gz/.tgz`, `.tar.xz`, `.tar.zst`, `.tar.bz2`, `.7z`
- Create `.zip` (with optional AES-256 password), `.tar`, `.tar.gz`, `.tar.zst`
- Extract all supported formats above, including password-protected zips and 7z
- "Test archive" — full CRC-32 verification pass on zip entries
- English UI wired end-to-end; 18 other locale slots scaffolded in the language
  dropdown (`src/languages/`) as "coming soon" — same convention used in Speusis,
  so translation files can just be dropped in later

## What's new in v0.1.1
- **Windows Explorer context menu**, WinRAR-style — right-click any file/folder → "Zarc" submenu:
  "Add to archive...", "Add to \"name\".zip", "Compress and email...". Right-clicking a `.zip`/`.7z`
  gets "Extract Here" and "Open with Zarc" too. Registered by `src-tauri/installer-hooks.nsh` at
  install time (needs `installMode: perMachine`, i.e. the installer runs elevated).
- **"Archive name and parameters" dialog** (`src/AddArchiveDialog.tsx`) — tabbed like WinRAR's:
  General (name, format, compression level), Advanced (AES-256 password + confirm), Options
  (test-after / delete-after-archiving), Comment. The "Add" toolbar button and the Explorer
  "Add to archive..." entry both open this now instead of silently zipping.
- **CLI launch-action parsing** (`src-tauri/src/lib.rs`) — the app now understands
  `--add`, `--add-default`, `--add-mail`, `--extract-here` argv flags (what the context menu
  commands invoke it with) and a bare archive path (double-click open), and hands that off to
  the frontend once on startup.
- **UI pass**: CSS custom properties for the whole palette (`src/App.css` `:root`), gradient
  title text, hover/press micro-motion on every button, staggered fade-in on table rows,
  animated modal (backdrop fade + scale-in), a spinner for in-flight operations.

## Roadmap to actually beat WinRAR (not yet built)
- [ ] `.7z` **writing** (currently read/extract only — `sevenz-rust`'s writer is younger than its reader)
- [ ] True `.rar` extraction via `unrar` bindings (RAR is proprietary — we can never *create* `.rar`, only read it, same as every other non-WinRAR tool)
- [ ] Zarc's own encrypted container format: AES-256-GCM + Argon2id key derivation for
      per-file E2E encryption (stronger and more modern than legacy ZipCrypto, on par
      with zip's AES mode but with authenticated encryption + zeroized key material)
- [ ] Multi-threaded compression via `rayon` (chunk-level parallel deflate/zstd — WinRAR is still largely single-threaded per file)
- [ ] Drag-and-drop from OS file explorer straight into the entry table
- [ ] In-app archive browsing without full extraction (stream individual files out)
- [ ] Split/multi-volume archives
- [ ] Self-extracting archive (.exe) builder
- [ ] "Delete files after archiving" is a checkbox in the dialog now but doesn't actually delete yet — deliberately left inert since it's destructive; wire it up once there's a confirm step
- [ ] "Compress and email..." currently just reveals the new zip in the file manager — no mail client attach hookup yet, Windows has no reliable API for that without knowing the installed mail client
- [ ] Real i18n loader (swap the inline `en.json` import for dynamic per-locale JSON loading once translations exist) + localize the Add-archive dialog itself (still hardcoded English strings)
- [ ] Single-instance enforcement (`tauri-plugin-single-instance`) so rapid multi-select context-menu clicks don't spawn N windows

## Project layout
```
zarc/
├─ src/                  React frontend
│  ├─ languages/en.json  UI strings (only translated locale so far)
│  ├─ i18n.ts            locale list + t() hook
│  ├─ api.ts             typed wrapper around Tauri invoke()
│  └─ App.tsx            toolbar + entry table + language switcher
└─ src-tauri/
   ├─ src/archive.rs     format detection, list/create/extract/test
   ├─ src/lib.rs         Tauri commands + app builder
   └─ tauri.conf.json    900x600 default window, NSIS/MSI/deb/AppImage/dmg bundling
```

## Security notes
- Zip password protection uses the `zip` crate's built-in AES-256 mode (`aes-crypto` feature) — not legacy ZipCrypto, which is trivially breakable.
- No telemetry, no network calls anywhere in this codebase.
- CSP is locked down in `tauri.conf.json`; only `dialog`/`fs`/`shell` capabilities are granted, and `fs` is scoped to explicit user-picked paths (see `capabilities/default.json`) rather than broad filesystem access.
