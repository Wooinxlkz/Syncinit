# Tugur v0.1.4

Tugur (formerly Zarc) is a fast, open, modern archive manager built with
Rust, Tauri v2 and React. It is designed as a practical WinRAR alternative
with a native Windows Explorer workflow and a branded `.arc` format.

## What's new in v0.1.4
- Rebrand: Zarc → Tugur, new app icon/logo throughout (taskbar, window,
  installer, file associations)
- Custom-branded NSIS installer: `setup.exe` now uses the new icon, plus a
  header/sidebar image built from the same logo (`src-tauri/installer/`)

## What's new in v0.1.3

- **Password prompt when opening** — encrypted `.arc` and ZIP archives now
  authenticate before their contents are displayed, with retry support for an
  incorrect password. Extraction also prompts and retries when needed.
- **Windows Explorer registration repair** — Tugur registers its per-user
  Explorer commands at startup as well as writing them from the NSIS installer,
  so context-menu actions work for unpacked and development builds too.
- **In-app context menus** — right-click an archive entry or empty archive area
  to add files, open an archive, extract, test, select all entries, or clear the
  current selection.

- **Native `.arc` archives** — `.arc` is a ZIP-compatible container with Tugur's
  bundled logo, AES-256 password support, archive comments and normal ZIP
  interoperability.
- **Explorer multi-select actions** — select one or many files/folders and use
  Tugur → Add to archive..., Add to .arc archive, or Compress and email....
- **Archive file integration** — `.arc` files open with Tugur and expose
  Extract Here and Open with Tugur in Explorer. ZIP and 7z keep their existing
  Tugur actions.
- **Smart store** — JPEG, PNG, MP4, PDF, ZIP, 7z and other formats that are
  already compressed are stored without wasting time recompressing them.
- **Faster and safer creation** — buffered I/O, a 128 KiB read buffer, atomic
  temporary output and ZIP path traversal protection during extraction.
- **Archive comments** — comments are now written into the archive and shown
  when an archive is open.
- **Delete after archiving** — implemented with an explicit native warning
  confirmation and a backend safety check that refuses to delete a source
  directory containing the new archive.

## Supported formats

- Read/extract: `.arc`, `.zip`, `.tar`, `.tar.gz`/`.tgz`, `.tar.xz`/`.txz`,
  `.tar.zst`, `.tar.bz2`/`.tbz2`, and `.7z`
- Create: `.arc`, `.zip`, `.tar`, `.tar.gz`, and `.tar.zst`

Tugur does not create proprietary `.rar` files. The `.arc` format is the
branded, open ZIP-compatible alternative: it is readable by Tugur and can be
opened by ZIP tools that inspect file signatures rather than extensions.

## Run it

```bash
bun install
bun run tauri dev
```

Build release bundles:

```bash
bun run tauri build
```

The Windows NSIS installer registers the Explorer verbs and the `.arc` file
association. Tugur also refreshes per-user registrations when it starts, which
helps when running an unpacked executable. Install per-machine for the full
installer integration.

## Stack

- Backend: Rust, Tauri v2, `zip`, `sevenz-rust`, `tar`, `flate2`, `xz2`,
  `zstd`, and `bzip2`
- Frontend: React 18 + TypeScript + Vite
- Security: AES-256 ZIP encryption, buffered atomic archive creation, and
  safe extraction paths
- No telemetry or network calls

## Project layout

```text
tugur/
├─ src/                  React frontend
│  ├─ AddArchiveDialog.tsx
│  ├─ PasswordDialog.tsx
│  ├─ api.ts
│  └─ App.tsx
└─ src-tauri/
   ├─ src/archive.rs     format detection, list/create/extract/test/delete
   ├─ src/lib.rs         Tauri commands and launch-action parsing
   ├─ installer-hooks.nsh
   └─ tauri.conf.json
```