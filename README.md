# Zarc v0.1.0

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
- [ ] Context-menu / shell integration (Explorer "Add to archive…" equivalent)
- [ ] Self-extracting archive (.exe) builder
- [ ] Real i18n loader (swap the inline `en.json` import for dynamic per-locale JSON loading once translations exist)

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
