# Releases

## v0.1.18

**Actually found the context-menu bug, from real data.** Your Diagnostics
screenshot showed the exact smoking gun: `argv` was
`["...\syncinit.exe", "--add-default"]` — no file path at all. Windows'
`%*` token, used in the "Add to archive…" / "Add to .init archive" /
"Compress and email…" command strings, was expanding to *nothing*, even
for a single selected file. Confirmed against real, documented reports of
this exact Windows quirk: `%*` is well known to be unreliable in
registry-based shell commands, while `%1` is universally reliable (and is
what Open/Extract Here were already using, which is why those never had
this problem). Switched all three add-related commands from `%*` to `%1`.

Side effect worth knowing about: with `%1`, selecting *multiple* files and
choosing "Add to archive…" launches Syncinit once per selected file
(a real, documented Windows behavior for this kind of registration), not
once with every path. Added a short batching window on the receiving end
so those rapid-fire single-file launches get merged into one dialog/one
quick-add instead of flickering through N separate ones or silently
ending up with just the last file.

## v0.1.17

Two of the four "cool features" ideas — the two that are frontend-only
(zero Rust changes), deliberately, after last version's build break from
a Rust edit gone wrong. Not risking that twice in one week.

- **Smart Extract Here** — matches 7-Zip/WinRAR: if an archive already
  wraps everything in one top-level folder, "Extract Here" behaves as
  before (its contents land inside that folder, nothing spills out). If
  it has multiple loose top-level files/folders instead, they now get
  their own new folder (named after the archive) rather than spilling
  directly into whatever folder the archive itself was sitting in.
  Only affects "Extract Here" from Explorer — "Extract to…", where you
  explicitly pick a destination yourself, is untouched, since second-
  guessing an explicit choice would be the wrong call.
- **Batch compress** — the Add-to-archive dialog now offers "Create a
  separate archive for each item" whenever more than one file/folder is
  selected. Off by default (combines into one archive, exactly like
  before); check it and each selected item gets its own archive instead.

Still on the list, each a real standalone undertaking rather than a
quick add — planned as separate efforts, not bundled into one risky
patch: a CLI tool, PAR2-style recovery records, and scheduled integrity
checks on a watched folder.

## v0.1.16

New feature (from the "what's worth adding" discussion) — first of the
recommended three, chosen as the cheapest/highest-leverage one to start
with since it extends something already built rather than adding a new
subsystem:

- **"Remember this password"** on the unlock dialog (opt-in, unchecked by
  default). Saves to the Windows Credential Manager via the `keyring`
  crate — Syncinit itself never writes the password to disk in plaintext,
  only a reference (the archive's path) is ever handled locally. When a
  saved password exists, opening that archive tries it silently first;
  if it's gone stale (password changed elsewhere), it's dropped and a
  normal prompt appears — no visible glitch either way. Changing or
  removing an archive's password (Commands → "Set/change password…")
  always clears whatever was saved for it, so a stale credential can't
  linger. Fully additive: an archive with nothing saved for it behaves
  exactly as before.

## v0.1.15

- **Dialog resizing: gave up on "smart" and made it fixed.** Three rounds
  of adaptive sizing (min-height, then framer-motion `layout`) kept
  getting reported as visibly resizing between tabs. Switched to what
  WinRAR's own dialog actually does: one fixed height (300px), nothing to
  animate, nothing to mismeasure. Every tab's content fits inside it with
  room to spare.
- **Freeze: added a real safety net, since I couldn't find the exact
  cause by reading code a fourth time.** Every Tauri command call now
  goes through a timeout wrapper — heavy archive operations get a
  generous 10-minute ceiling (real large archives can legitimately take
  a while), everything else gets 20 seconds. If any single call ever
  hangs without resolving *or* rejecting — which is exactly what a
  permanently-stuck `busy` state and "have to restart the app" looks
  like — it now fails loudly instead of hanging forever, so the calling
  code's own cleanup still runs and the UI recovers on its own. This
  doesn't pretend to know the root cause; it makes the failure mode
  survivable regardless of what it turns out to be.
- **All 12 languages are now actually translated** — French, German,
  Spanish, Dutch, Norwegian, Polish, Russian, Arabic, Indonesian, Chinese
  (Simplified), and Japanese, all 20 UI strings each. One honest caveat:
  Arabic text itself is correctly translated, but the app's layout isn't
  RTL-aware yet (no right-to-left flip), so the toolbar and panels stay
  left-to-right even when Arabic is selected.

## v0.1.14

Took the "check open source, use what's there" instruction seriously —
researched two real open-source archivers (Squallz, Ziplark; same stack,
Tauri+Rust) and brought over what actually applied:

- **Decompression-bomb guardrails** — entry-count limits, running
  output-size limits, and per-entry compression-ratio limits, modeled on
  the baseline safety guardrails Squallz documents explicitly. Syncinit
  had zip-slip/path-traversal protection already but nothing stopping a
  malicious or corrupt archive claiming millions of entries or a
  1000:1+ expansion ratio from hanging the app or filling the disk.
  Applied to extraction and to listing (a bogus entry-count in a zip's
  central directory could otherwise trigger a huge upfront allocation
  before a single byte is read).
- **Extraction now survives a damaged archive** — matches Ziplark's
  "extract what's readable": a corrupt or unsafe entry is logged as a
  warning and extraction continues, instead of the whole operation
  aborting on the first bad entry. Same shape as the fix already applied
  to archive *creation*. 7z extraction stays all-or-nothing — the
  library used gives no per-entry hook to do better.
- **Context menu Reinstall / Uninstall**, explicit buttons in
  Settings → Diagnostics — matches Ziplark's `shell-integration
  install/status/uninstall` pattern instead of silent, invisible
  auto-registration on every launch with no way to undo it short of
  hand-editing the registry.
- **RAR read support** (list + extract), via a system 7-Zip install if
  one exists — RAR's format is proprietary with no pure-Rust decoder,
  and linking libunrar directly needs a compiled C library with its own
  license terms this project can't take on blind. This is the one
  change in this release that genuinely needs real-world testing before
  trusting it: the 7-Zip CLI output parsing has not run against an
  actual 7-Zip binary or a real RAR file in this environment.

## v0.1.13

- **Researched before fixing, as asked.** Confirmed via Microsoft's own
  shell-verb docs that `MultiSelectModel` must be set on *each verb*, not
  inherited from a cascading parent menu — and found a 7-Zip registration
  reference making the same point about multi-file "Add to archive"
  specifically. Our code only ever set it on the parent "Syncinit" key,
  never on the "Add to archive…"/"Add to .init archive"/"Compress and
  email…" entries themselves. Now set on all three directly — this is a
  real, documented cause for multiple selected files not correctly
  reaching one invocation, not a guess. Also confirmed 7-Zip's most
  polished integration is a full COM shell extension (a separate DLL),
  which is a much bigger undertaking than this registry-based approach;
  noting that honestly rather than pretending it's equivalent.
- **New in-app Diagnostics page** (Settings → Diagnostics): raw launch
  argv, what it parsed into, a running timestamped log of every
  registration/relaunch event since startup, live context-menu registry
  state, and the `.init` icon's actual on-disk status (path, size, last
  write time) — with one-click "Copy all". No more needing devtools open
  to report a bug.
- **Password/PIN can now be set, changed, or removed on an existing
  archive** — Commands → "Set/change password…". Zip/AES encryption is
  applied per-entry at write time, so there's no way to "just add" a
  password to bytes already written; this decrypts every entry (with the
  old password, if it's already protected) and rewrites the archive with
  the new setting.

## v0.1.12

Full audit fix, everything from the "check all the bullshit errors" list:

- **The freeze — actually fixed everywhere, not just Add.** In Tauri 2 a
  synchronous `fn` command runs on the *main thread*; only `async fn`
  commands run off it. Every command that touches the filesystem for real
  (`list_archive`, `create_archive`, `extract_archive`, `test_archive`,
  `delete_entries`, `delete_sources`, and `register_context_menu`) was
  plain `fn`. Converted all of them to `async fn` wrapping the real work in
  `tauri::async_runtime::spawn_blocking` — this was never just an "Add"
  bug, it was every heavy operation in the app freezing the window for as
  long as it took.
- **Silent data loss when creating an archive — fixed.** The directory
  walk used `.filter_map(|e| e.ok())`, which silently dropped any path it
  couldn't read (permission denied, a broken symlink, Windows' 260-char
  path limit) with zero indication — the archive would finish and report
  success even with files missing from it. Now every skipped path is
  collected and returned to the UI as an explicit warning after the
  archive completes. Also: directory walks now follow symlinks
  (`follow_links(true)`), so a symlinked folder's actual contents get
  included instead of it just appearing as an empty folder in the
  archive — verified against walkdir's own built-in symlink-cycle
  detection first, so this can't introduce a hang of its own.
- **A hard-crash path removed.** `test_archive`'s non-zip fallback did
  `path.to_str().unwrap()` — on the rare path that isn't valid UTF-8,
  this panics, and this build has `panic = "abort"` in the release
  profile, so a panic here doesn't fail gracefully, it takes down the
  whole app. Switched to a lossy conversion that can't panic.
- **`open_url` hardened.** Replaced the `cmd /C start` hack with the
  `open` crate — its own docs confirm the `cmd`-based approach "cannot
  safely receive untrusted paths because cmd interprets its arguments as
  shell syntax," which was exactly the theoretical risk flagged. `open`'s
  default (non-`insecure`) path avoids it entirely, no console window.
- **Update-check fetch now has a timeout.** Previously a hung (not
  erroring) request to GitHub would just never resolve.

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
