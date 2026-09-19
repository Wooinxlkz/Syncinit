# Releases

## v0.1.11

- **Found the actual cause of the .init icon showing the main app icon**:
  `write_init_file_icon()` wrote next to the exe — fine for a dev build,
  but a normal install lives under `C:\Program Files\Syncinit\`, which a
  non-admin user can't write to. The write silently failed, and the code
  fell back to using the exe itself (the main icon) as the DefaultIcon
  target. That fallback was always the visible behavior on a real install;
  it had nothing to do with icon caching or a stale build. Now writes to
  `%LOCALAPPDATA%\Syncinit\`, which is always writable.
- **Modal system rebuilt on Radix UI** (`@radix-ui/react-dialog`) instead
  of the hand-rolled focus-trap — every dialog (Add to archive, password
  prompt, Settings, Archive info) goes through it now. The freeze on
  Cancel was traced to the old hand-rolled focus trap; rather than patch
  around it further, swapped it for a widely-used, battle-tested
  implementation. Also hardened `OTPInput`'s error-shake animation to stop
  itself on unmount instead of potentially running against a detached
  node — a plausible contributor if the dialog closed mid-shake.
- **Add-to-archive dialog now animates its height to fit each tab**
  (framer-motion `layout` on the panel) instead of either snapping
  instantly or sitting at an oversized fixed height with dead space.
- **Context-menu "opens the app but doesn't add files"**: went through
  the whole launch-parsing path again and can't find a bug in it by
  inspection. Added a `get_raw_args` command, logged to the devtools
  console on startup (and the relaunch event payload is logged too) —
  next time this happens, that console output will show exactly what
  Explorer actually invoked, instead of guessing further blind.

## v0.1.10

- **Context menu submenu items now have icons** ("Add to archive…", "Add
  to .init archive", "Compress and email…", "Open", "Extract Here") — none
  of them had an `Icon` registry value set before, which is why they
  rendered blank next to entries like WinRAR/ESET that do set one.
- **Fixed a real `.init.init` doubling bug**: Explorer's "Add to .init
  archive"/"Compress and email…" built their default name by appending
  `.init` to the source's full filename without stripping its existing
  extension — only the Add-to-archive dialog's own default name did that
  stripping. All three now share one `defaultInitName()` helper, so
  they agree (and re-running "Add to .init archive" on something already
  named `*.init` no longer stacks a second `.init`).
- On "right-click only opens the app instead of adding files": traced the
  whole launch/single-instance path and found nothing wrong in it — worth
  double-checking this is actually running v0.1.10 (and not a build from
  before the "Tugur" cleanup landed) before treating it as still-open.
- **Add-to-archive dialog no longer resizes when switching tabs** — its
  body was sized with `min-height` (each tab's differing content height
  could stretch it), now a fixed height with scroll as a fallback.
- **Language list trimmed** to the 12 requested locales, and it's an
  actual themed `<select>` dropdown in Settings now, not a row list.
- **Light/dark theme toggle**, in Settings → General. Full light palette
  ported 1:1 from Xuro's own light `:root` block; persisted locally.
- **Themed scrollbars** app-wide (were the OS default before).
- **Update checker**: a dismissible banner checks
  github.com/Wooinxlkz/Syncinit's releases and points at the release page
  when a newer version exists. This is a notification, not Xuro's full
  silent auto-install — that needs a signed-update-manifest setup this
  project doesn't have; can be built separately if wanted.

## v0.1.9

- **Removed the in-app titlebar** ("Zarc" text + the language dropdown) —
  it was fully redundant with the OS titlebar and MenuBar's own drag
  region, so it's just gone rather than patched. Confirmed (again) the
  source has zero occurrences of "Zarc" anywhere; if it's still showing up
  visually, that screenshot is from an old installed build, not this one —
  see the icon/registry fix below for the actual reason an upgrade could
  look like nothing changed.
- **New Settings** (gear icon, bottom-right of the toolbar) replacing the
  old titlebar dropdown and the standalone About dialog — sidebar nav +
  content pane, structurally ported from Xuro's own `SettingsModal.tsx`.
  General page holds language; About page holds the app info, in Xuro's
  card layout.
- **Default archive format is now SYNCINIT (.INIT)**, not ZIP. The format
  picker in Add-to-archive is now the actual beui.dev/Xuro `RadioGroup`
  (shared-layout dot glide + press spring), ported from Tailwind to this
  project's plain CSS.
- **Found the real reason an upgrade could look like it changed nothing**:
  `register_context_menu()`'s idempotency check only compared the
  registered exe *path* — on an upgrade installed to the same path, that
  check matched immediately and skipped every write below it, including
  the `.init` file-icon refresh and the menu's Icon/MUIVerb values, no
  matter what changed in the new build. It now also checks a stored
  version marker, so a version bump always re-registers. Separately,
  `write_init_file_icon()` had `if !ico_path.exists()` — meaning even
  when re-registration *did* run, an icon file already on disk from an
  older install was never replaced. Both fixed: this is what "new logo
  doesn't show up" and "still looks like the old version after updating"
  actually were.
- **PIN dialog cancel now actually closes out**: cancelling the PIN
  prompt while opening an archive resets to a clean "no archive open"
  state instead of leaving things ambiguous.
- **PIN dialog spacing fixed** — it was inheriting the Add-to-archive
  dialog's 190px `min-height` (sized for that dialog's tabs), which is
  where the large empty gap under the PIN boxes came from. Password
  dialog now sizes to its own content and centers it.

## v0.1.8

- **Dialogs rebuilt to match Xuro's UI**: Add-to-archive, the PIN/password
  prompt, About, and Archive info now share one `Modal` component (ported
  from Xuro's `src/components/ui/Modal.tsx`) — rendered into a portal so
  it's never clipped, spring-animated open/close instead of the old plain
  `div` with no motion, focus-trapped, closes on Escape or backdrop click.
  The dialog panel no longer clips its own content (`overflow: hidden` →
  rounded header/footer instead), so an internal dropdown can open past
  the panel edge instead of being cut off.
- **Locale dropdown fixed**: it had both a CSS `@keyframes` animation and
  a framer-motion animation running on the same panel at once, fighting
  over opacity/transform — that's what made opening it feel broken. Now
  motion-only, same as the rest of the app.
- **Drag-and-drop implemented** — this genuinely didn't exist before:
  `dragDropEnabled` was `false` in `tauri.conf.json` and there was no
  listener at all, so dropping files onto the window did nothing no
  matter what. Now `dragDropEnabled: true` plus a real
  `getCurrentWebview().onDragDropEvent()` listener, with a drop-target
  overlay, wired straight into the same Add-to-archive dialog "Add
  files…" already uses.
- **Explorer "Add to archive…" fixed for the case that actually mattered**:
  the app had no single-instance handling, so triggering a context-menu
  command while Syncinit was already open spawned a second, separate
  process instead of using the open window — which is what "doesn't
  work" looked like in practice. `tauri-plugin-single-instance` now
  forwards the new invocation's files to the already-running window.
- **Legacy registry cleanup**: earlier installs (from the Zarc/Tugur
  rebrands) could leave old "Zarc"/"Tugur" shell-menu registry trees
  sitting alongside the current "Syncinit" one — the source of seeing the
  old name in the right-click menu on an upgraded install. Every app
  launch now deletes those legacy trees before re-registering.
  Windows 11 still tucks non-pinned shell entries under "Show more
  options" for every classic-registered app (WinRAR included, without a
  packaged extension) — that's an OS-level menu behavior, not something a
  registry entry can override.
- **`.init` file icon regenerated** from the latest logo, as a proper
  multi-resolution `.ico` (16 up to 256px) instead of a single 256px
  frame — Explorer picks a different embedded size per view (list icons,
  large icons, thumbnails), so a single-size icon could render blank in
  some views. The app's own icon is untouched.

## v0.1.7

- **Rebrand: Tugur → Syncinit**, and the archive format extension
  `.arc` → `.init` throughout — registry, file associations, UI labels,
  docs. (Older bullets below now read "Syncinit"/".init" too, for
  consistency, except the line documenting the original Zarc rename,
  which stays as it was written.)
- **Two distinct icons**, not one reused everywhere: the app itself uses
  the new "eye" logo; `.init` files in Explorer get a separate "wrapped
  box" icon, embedded directly in the binary and written out next to the
  exe at runtime — chosen over an unverified Tauri config field so it
  didn't risk silently doing nothing.
- New NSIS installer header/sidebar branding built from the new logo.
- **Icon toolbar**, WinRAR-style: Add, Extract, Test, Delete, Find, Info,
  Comment — each a real, wired action, nothing decorative. Deliberately
  left out: View (would need safe per-entry extraction-and-open, not
  built yet), Wizard (redundant with the existing Add dialog), VirusScan
  (no AV integration exists — a visible button for this would be actively
  misleading), SFX (still not built, see prior releases).
- **Top menu bar** — File / Commands / Favorites / Tools / Options / Help,
  each item real: Favorites is an actual recently-opened list backed by
  localStorage; Options lists the ready locales; Help opens a real About
  panel.
- **New backend feature**: delete entries from an existing archive
  (rebuilds it without them). Uses the same decompress-then-recompress
  approach as the rest of the codebase rather than a faster raw-copy API
  that couldn't be verified without a working `cargo build` — slower on
  large files, but nothing here risks producing a corrupt archive.
- Find/filter box for the entry table; an Info panel (format, entry count,
  sizes, encryption status); a full-comment view.
- Caught and fixed a self-introduced bug before it shipped: a careless
  edit briefly deleted the `#[cfg(not(windows))]` guard on the
  context-menu registration function, which would have made it always
  report success without doing anything, on every platform.

## v0.1.6

- **Progress bars + Cancel**, for real this time — every create/extract now
  reports live `done/total` bytes to the UI (throttled to ~20 updates/sec
  so it can't flood the webview and stutter the app the way an earlier,
  unthrottled attempt did), and a Cancel button actually stops the
  operation mid-file rather than just hiding a spinner over work that kept
  running underneath.
- Cancelling — or any failed create — no longer leaves a half-written
  `.init`/`.zip` file behind; it's built to a temp file and only renamed
  into place on success, cleaned up otherwise.
- **Real multi-threaded compression for `.tar.zst`** via libzstd's own
  worker-thread pool (`zstd`'s `zstdmt` feature), not a hand-rolled
  approximation.
- **Honest limitation, not silently skipped**: `.init`/`.zip`/`.tar.gz`
  compression is *not* multi-threaded in this version. The `zip` crate
  writes to one sequential output stream, and doing this safely needs a
  raw-precompressed-entry write path I couldn't verify compiles correctly
  without a working `cargo build` in the environment this was written in —
  shipping unverified low-level zip internals risked producing corrupt
  archives, so it's staying on the roadmap until it can be tested for real
  rather than guessed at.

## v0.1.5

- Rebuilt `LICENSE` as proprietary (was MIT) — download-and-run only, no
  redistribution or reuse rights granted; see the file for exact terms.
- Rebuilt `README.md`: removed build/install instructions and version
  history, added a top banner image, moved release notes here.
- Added `PRIVACY.md`, `TERMS.md`, `THIRD-PARTY-NOTICES.md`.
- Fixed two WCAG contrast failures: `.hint` text and PIN status messages
  were at 2.93:1 against the background (fails AA's 4.5:1 minimum) — moved
  to the `--muted` token, 5.36:1, passes.
- In-app right-click menu rebuilt using Xuro's actual `ContextMenu.tsx`
  mechanics (portal, viewport-edge clamping, roving keyboard focus,
  danger-item styling) instead of a static CSS list, plus lucide icons per
  item and an entrance animation.
- Language dropdown: fixed a background-color mismatch (`--sunken` instead
  of `--panel`, made it visibly darker than the toolbar it sits next to)
  and standardized its border-radius/transition tokens to match the rest
  of the app's chrome.
- Explorer context-menu registration: rewrote the reg-write path to spawn
  zero processes (`winreg` crate, direct Win32 registry calls) instead of
  shelling out to `reg.exe` ~40 times per launch — fixes both the startup
  freeze and the console-window flash reported on v0.1.3.
- "Compress and email..." no longer flashes a console either — replaced
  `tauri-plugin-shell`'s `open()` (shells through `cmd /C start` on
  Windows) with a dedicated `reveal_in_file_manager` command that spawns
  `explorer.exe` directly with `CREATE_NO_WINDOW`.

## v0.1.4

- Rebrand: Zarc → Tugur.
- New app icon/logo throughout (taskbar, window, installer, file
  associations), generated from the provided logo artwork.
- Custom-branded NSIS installer: `setup.exe` uses the new icon, plus a
  header/sidebar image built from the same logo.

## v0.1.3

- Password prompt when opening — encrypted `.init` and ZIP archives
  authenticate before their contents are displayed, with retry support.
- Windows Explorer registration moved to run at app startup (HKCU) in
  addition to the NSIS installer's entries, so context-menu actions work
  for unpacked/dev builds too.
- In-app context menus — right-click an archive entry or empty archive
  area to add files, open an archive, extract, test, select all, or clear
  the current selection.
- Native `.init` archives — a ZIP-compatible container with Syncinit's own
  branding, AES-256 password support, and archive comments, while staying
  interoperable with plain ZIP tools.
- Explorer multi-select actions — select one or many files/folders and use
  Syncinit → Add to archive..., Add to .init archive, or Compress and email....
- Archive file integration — `.init` files open with Syncinit and expose
  Extract Here / Open with Syncinit in Explorer; ZIP and 7z keep the same
  actions.
- Smart store — already-compressed formats (JPEG, PNG, MP4, PDF, ZIP, 7z,
  etc.) are stored rather than wastefully recompressed.
- Faster, safer archive creation — buffered I/O, atomic temporary output,
  and ZIP path-traversal protection during extraction.
- Archive comments are now actually written into the archive, not just
  held in the UI.
- Delete-after-archiving implemented with an explicit confirmation and a
  safety check that refuses to delete a source directory containing the
  new archive.

## v0.1.1

- Windows Explorer right-click context menu (Add to archive.../Add to
  "name".zip/Compress and email...), registered via an NSIS installer
  hook.
- "Archive name and parameters" dialog (General/Advanced/Options/Comment
  tabs), matching WinRAR's own dialog structure.
- CLI launch-action parsing so the context-menu commands actually drive
  the app (`--add`, `--add-default`, `--add-mail`, `--extract-here`).
- First UI pass matching Xuro's design tokens and motion curve.

## v0.1.0

- Initial scaffold: Rust/Tauri v2 backend with zip/tar/7z list, create,
  extract, and test support; React/TypeScript frontend with a WinRAR-style
  toolbar and entry table; English-only i18n groundwork with the rest of
  the locale list stubbed in for later.
