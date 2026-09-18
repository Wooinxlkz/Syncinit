import { useEffect, useState, type MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm, open } from "@tauri-apps/plugin-dialog";
// (no plugin-shell import — reveal_in_file_manager on the Rust side avoids
// the console-flash the shell plugin's open() causes on Windows)
import { api, formatBytes, type ArchiveSummary } from "./api";
import { useI18n, SUPPORTED_LOCALES } from "./i18n";
import { LocaleDropdown } from "./LocaleDropdown";
import { MenuBar } from "./MenuBar";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import {
  FolderInput,
  FolderOpen,
  ListChecks,
  Package,
  SquareCheck,
  SquareX,
  TestTubeDiagonal,
  X,
  Plus,
  FolderOutput,
  FlaskConical,
  Trash2,
  Search,
  Info,
  MessageSquare,
  HelpCircle,
  Star,
  Settings,
  Wrench,
  FileArchive,
} from "lucide-react";
import AddArchiveDialog, { type ArchiveDialogResult } from "./AddArchiveDialog";
import PasswordDialog from "./PasswordDialog";
import { Modal } from "./Modal";
import "./App.css";

type LaunchAction =
  | { mode: "add-dialog"; paths: string[] }
  | { mode: "add-default"; paths: string[] }
  | { mode: "add-and-mail"; paths: string[] }
  | { mode: "extract-here"; paths: string[] }
  | { mode: "open-archive"; path: string }
  | { mode: "none" };

type PasswordRequest = {
  path: string;
  purpose: "open" | "extract";
  destination?: string;
};

type ContextMenuState = {
  x: number;
  y: number;
  entry?: string;
};

function dirname(p: string) {
  const clean = p.replace(/[\\/]+$/, "");
  const idx = Math.max(clean.lastIndexOf("/"), clean.lastIndexOf("\\"));
  return idx >= 0 ? clean.slice(0, idx) : clean;
}
function basename(p: string) {
  const clean = p.replace(/[\\/]+$/, "");
  const idx = Math.max(clean.lastIndexOf("/"), clean.lastIndexOf("\\"));
  return idx >= 0 ? clean.slice(idx + 1) : clean;
}

function archiveExtension(format: ArchiveDialogResult["format"]) {
  return format === "init"
    ? ".init"
    : format === "targz"
      ? ".tar.gz"
      : format === "tarxz"
        ? ".tar.xz"
        : format === "tarzst"
          ? ".tar.zst"
          : format === "tarbz2"
            ? ".tar.bz2"
            : `.${format}`;
}

function normalizeDestination(name: string, format: ArchiveDialogResult["format"]) {
  const extension = archiveExtension(format);
  const archiveNamePattern = /\.(arc|zip|tar|tar\.gz|tgz|tar\.xz|txz|tar\.zst|tar\.bz2|tbz2|7z)$/i;
  if (archiveNamePattern.test(name)) return name.replace(archiveNamePattern, extension);
  return `${name}${extension}`;
}

function isPasswordError(error: unknown) {
  const message = String(error).toLowerCase();
  return message.includes("password") || message.includes("encrypted");
}

function isCancelledError(error: unknown) {
  return String(error).toLowerCase().includes("cancelled");
}

export default function App() {
  const { t, locale, setLocale } = useI18n();
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [summary, setSummary] = useState<ArchiveSummary | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [dialogSources, setDialogSources] = useState<string[] | null>(null);
  const [passwordRequest, setPasswordRequest] = useState<PasswordRequest | null>(null);
  const [passwordError, setPasswordError] = useState("");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [filterText, setFilterText] = useState("");
  const [findOpen, setFindOpen] = useState(false);
  const [infoOpen, setInfoOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [favorites, setFavorites] = useState<string[]>(() => {
    try {
      return JSON.parse(localStorage.getItem("syncinit:favorites") ?? "[]");
    } catch {
      return [];
    }
  });

  useEffect(() => {
    const unlisten = listen<{ done: number; total: number }>("syncinit://progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Native OS drag-and-drop, both directions:
  //  - dragging files FROM outside the window (Explorer, desktop) onto it
  //    opens the Add-to-archive dialog with the dropped paths, same as
  //    "Add files…"/right-click "Add to archive…";
  //  - dragging a row OUT of the window (onto Explorer) is handled by
  //    ArchiveTable's own draggable rows using Tauri's startDrag, not here.
  // This needs `dragDropEnabled: true` in tauri.conf.json — with it false,
  // the webview swallows OS drag events entirely and neither this listener
  // nor a plain HTML5 onDrop ever fires, which is why dropping did nothing.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          const kind = event.payload.type;
          if (kind === "enter" || kind === "over") {
            setDragActive(true);
          } else if (kind === "leave") {
            setDragActive(false);
          } else if (kind === "drop") {
            setDragActive(false);
            const paths = event.payload.paths;
            if (paths && paths.length > 0) {
              setDialogSources(paths);
            }
          }
        })
        .then((fn) => {
          if (cancelled) fn();
          else unlisten = fn;
        })
        .catch((err) => console.warn("Drag-and-drop listener failed to attach:", err));
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Pick up context-menu launches: "Add to archive...", "Add to X.init",
  // "Compress and email...", "Extract Here", or a double-clicked archive.
  useEffect(() => {
    // Register in HKCU as well as the installer HKCR entries. This makes the
    // Explorer menu work when running a development build or an unpacked exe.
    invoke<boolean>("register_context_menu")
      .catch((err) => console.warn("Context menu registration failed:", err))
      .finally(() => {
        invoke("debug_context_menu").then((info) =>
          console.log("Syncinit context menu registry state:", info)
        );
      });
    const closeMenu = () => setContextMenu(null);
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setContextMenu(null);
    };
    window.addEventListener("click", closeMenu);
    window.addEventListener("keydown", closeOnEscape);

    invoke<LaunchAction>("get_launch_action").then((action) => {
      if (action && action.mode !== "none") handleLaunchAction(action);
    });
    const unlistenRelaunch = listen<LaunchAction>("syncinit://relaunch-action", (event) => {
      handleLaunchAction(event.payload);
    });
    return () => {
      window.removeEventListener("click", closeMenu);
      window.removeEventListener("keydown", closeOnEscape);
      unlistenRelaunch.then((fn) => fn());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Shared by the initial launch action (get_launch_action, read once at
  // startup) and by "syncinit://relaunch-action" — emitted by the Rust
  // single-instance plugin handler when Explorer's "Add to archive…" /
  // "Add to X.init" / etc. is invoked while Syncinit is *already* running.
  // Without this, that second launch used to spawn a whole separate process
  // instead of reusing the open window, which looked like the context menu
  // command silently did nothing (the new window could open behind the
  // existing one, or the two would race on the same files).
  async function handleLaunchAction(action: LaunchAction) {
    try {
      if (action.mode === "add-dialog") {
        setDialogSources(action.paths);
      } else if (action.mode === "add-default") {
        await runCreate(action.paths, {
          name: `${basename(action.paths[0])}.init`,
          format: "init",
          level: 6,
          password: "",
          deleteAfter: false,
          testAfter: false,
          comment: "",
          smartStore: true,
        });
      } else if (action.mode === "add-and-mail") {
        const dest = await runCreate(action.paths, {
          name: `${basename(action.paths[0])}.init`,
          format: "init",
          level: 6,
          password: "",
          deleteAfter: false,
          testAfter: false,
          comment: "",
          smartStore: true,
        });
        if (dest) await invoke("reveal_in_file_manager", { path: dirname(dest) });
      } else if (action.mode === "extract-here") {
        const src = action.paths[0];
        await extractTo(src, dirname(src));
      } else if (action.mode === "open-archive") {
        await loadArchive(action.path);
      }
    } catch (err) {
      setStatus(String(err));
    }
  }

  async function loadArchive(path: string, password?: string) {
    setBusy(true);
    try {
      const data = await api.listArchive(path, password);
      setArchivePath(path);
      setSummary(data);
      setSelected(new Set());
      setPasswordRequest(null);
      setPasswordError("");
      setStatus(
        t("status.entries", { count: data.entries.length, size: formatBytes(data.total_uncompressed) })
      );
      setFavorites((prev) => {
        const next = [path, ...prev.filter((p) => p !== path)].slice(0, 8);
        localStorage.setItem("syncinit:favorites", JSON.stringify(next));
        return next;
      });
    } catch (err) {
      if (isPasswordError(err)) {
        setPasswordRequest({ path, purpose: "open" });
        setPasswordError(password ? "Incorrect password. Try again." : "");
      } else {
        setStatus(String(err));
      }
    } finally {
      setBusy(false);
    }
  }

  async function openArchive() {
    const file = await open({
      multiple: false,
      filters: [{ name: "Archives", extensions: ["init", "zip", "7z", "tar", "gz", "tgz", "xz", "zst", "bz2"] }],
    });
    if (file) await loadArchive(file as string);
  }

  async function runCreate(sources: string[], result: ArchiveDialogResult): Promise<string | null> {
    let destination = normalizeDestination(result.name.trim(), result.format);
    if (!destination.includes("/") && !destination.includes("\\")) {
      destination = `${dirname(sources[0])}/${destination}`;
    }
    setBusy(true);
    setProgress(null);
    try {
      await api.createArchive({
        destination,
        sources,
        level: result.level,
        format: result.format,
        password: result.password || null,
        smart_store: result.smartStore,
        comment: result.comment || null,
      });
      if (result.testAfter) await api.testArchive(destination);
      if (result.deleteAfter) {
        const approved = await confirm(
          `Delete ${sources.length === 1 ? "the original item" : `${sources.length} original items`} after the archive passes${result.testAfter ? " its test" : ""}? This cannot be undone.`,
          { title: "Syncinit — delete originals", kind: "warning" },
        );
        if (approved) {
          await api.deleteSources(sources, destination);
        }
      }
      await loadArchive(destination);
      return destination;
    } catch (err) {
      setStatus(isCancelledError(err) ? "Cancelled." : String(err));
      return null;
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }

  async function addFilesViaDialog() {
    const files = await open({ multiple: true });
    if (!files) return;
    setDialogSources(Array.isArray(files) ? files : [files]);
  }

  async function extractTo(source: string, destination: string, password?: string) {
    setBusy(true);
    setProgress(null);
    try {
      const count = await api.extractArchive(source, destination, password);
      setPasswordRequest(null);
      setPasswordError("");
      setStatus(t("toast.extracted", { count, dest: destination }));
    } catch (err) {
      if (isCancelledError(err)) {
        setStatus("Cancelled.");
      } else if (isPasswordError(err)) {
        setPasswordRequest({ path: source, purpose: "extract", destination });
        setPasswordError(password ? "Incorrect password. Try again." : "");
      } else {
        setStatus(String(err));
      }
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }

  async function extractCurrent() {
    if (!archivePath) return;
    const destination = await open({ directory: true });
    if (destination) await extractTo(archivePath, destination as string);
  }

  async function submitPassword(password: string) {
    if (!passwordRequest) return;
    const request = passwordRequest;
    if (request.purpose === "open") {
      await loadArchive(request.path, password);
    } else if (request.destination) {
      await extractTo(request.path, request.destination, password);
    }
  }

  async function testCurrent() {
    if (!archivePath) return;
    setBusy(true);
    try {
      await api.testArchive(archivePath);
      setStatus(t("toast.testPassed"));
    } catch (err) {
      setStatus(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function deleteSelectedEntries() {
    if (!archivePath || selected.size === 0) return;
    const names = Array.from(selected);
    const approved = await confirm(
      `Remove ${names.length === 1 ? `"${names[0]}"` : `${names.length} entries`} from the archive? This rebuilds the archive without them and cannot be undone.`,
      { title: "Syncinit — delete from archive", kind: "warning" }
    );
    if (!approved) return;
    setBusy(true);
    try {
      await api.deleteEntries(archivePath, names);
      await loadArchive(archivePath);
    } catch (err) {
      setStatus(String(err));
    } finally {
      setBusy(false);
    }
  }

  function toggleSelect(name: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(name) ? next.delete(name) : next.add(name);
      return next;
    });
  }

  function showContextMenu(event: MouseEvent, entry?: string) {
    event.preventDefault();
    event.stopPropagation();
    if (entry) {
      setSelected((previous) => previous.has(entry) ? previous : new Set([entry]));
    }
    setContextMenu({ x: event.clientX, y: event.clientY, entry });
  }

  function selectAllEntries() {
    if (!summary) return;
    setSelected(new Set(summary.entries.map((entry) => entry.name)));
    setContextMenu(null);
  }

  function buildMenuItems(menu: ContextMenuState): MenuItem[] {
    const items: MenuItem[] = [];
    if (menu.entry) {
      items.push({
        label: selected.has(menu.entry) ? "Unselect entry" : "Select entry",
        icon: selected.has(menu.entry) ? SquareX : SquareCheck,
        onSelect: () => toggleSelect(menu.entry!),
      });
    }
    items.push({ label: "Add files to archive…", icon: Package, onSelect: addFilesViaDialog });
    items.push({ label: "Open archive…", icon: FolderOpen, onSelect: openArchive });
    items.push({
      label: "Extract to…",
      icon: FolderInput,
      disabled: !archivePath,
      onSelect: extractCurrent,
    });
    items.push({
      label: "Test archive",
      icon: TestTubeDiagonal,
      disabled: !archivePath,
      onSelect: testCurrent,
    });
    items.push({
      label: "Select all entries",
      icon: ListChecks,
      disabled: !summary,
      onSelect: selectAllEntries,
    });
    if (selected.size > 0) {
      items.push({ label: "Clear selection", icon: X, danger: true, onSelect: () => setSelected(new Set()) });
    }
    return items;
  }

  return (
    <div className="app" onContextMenu={showContextMenu}>
      <header className="titlebar">
        <span className="app-name">{t("app.title")}</span>
        <LocaleDropdown locale={locale} onChange={setLocale} />
      </header>

      <MenuBar
        sections={[
          {
            label: "File",
            items: [
              { label: "Open archive…", icon: FolderOpen, onSelect: openArchive },
              { label: "Add files…", icon: Plus, onSelect: addFilesViaDialog },
              { label: "Exit", icon: X, onSelect: () => getCurrentWindow().close() },
            ],
          },
          {
            label: "Commands",
            items: [
              { label: "Add to archive…", icon: Package, onSelect: addFilesViaDialog },
              { label: "Extract to…", icon: FolderOutput, disabled: !archivePath, onSelect: extractCurrent },
              { label: "Test archive", icon: FlaskConical, disabled: !archivePath, onSelect: testCurrent },
              {
                label: "Delete from archive",
                icon: Trash2,
                danger: true,
                disabled: !archivePath || selected.size === 0,
                onSelect: deleteSelectedEntries,
              },
            ],
          },
          {
            label: "Favorites",
            items:
              favorites.length > 0
                ? favorites.map((fav) => ({
                    label: basename(fav),
                    icon: FileArchive,
                    onSelect: () => loadArchive(fav),
                  }))
                : [{ label: "No recent archives yet", disabled: true, onSelect: () => {} }],
          },
          {
            label: "Tools",
            items: [
              { label: "Find in archive", icon: Search, disabled: !summary, onSelect: () => setFindOpen((v) => !v) },
              { label: "Archive info", icon: Info, disabled: !summary, onSelect: () => setInfoOpen(true) },
              { label: "View comment", icon: MessageSquare, disabled: !summary?.comment, onSelect: () => setInfoOpen(true) },
            ],
          },
          {
            label: "Options",
            items: SUPPORTED_LOCALES.filter((l) => l.ready).map((l) => ({
              label: l.label,
              disabled: l.code === locale,
              onSelect: () => setLocale(l.code),
            })),
          },
          {
            label: "Help",
            items: [{ label: "About Syncinit", icon: HelpCircle, onSelect: () => setAboutOpen(true) }],
          },
        ]}
      />

      <div className="icon-toolbar">
        <button className="icon-toolbar-btn" onClick={addFilesViaDialog} disabled={busy}>
          <Plus size={20} strokeWidth={1.75} />
          {t("toolbar.add")}
        </button>
        <button className="icon-toolbar-btn" onClick={extractCurrent} disabled={busy || !archivePath}>
          <FolderOutput size={20} strokeWidth={1.75} />
          {t("toolbar.extract")}
        </button>
        <button className="icon-toolbar-btn" onClick={testCurrent} disabled={busy || !archivePath}>
          <FlaskConical size={20} strokeWidth={1.75} />
          {t("toolbar.test")}
        </button>
        <button
          className="icon-toolbar-btn danger"
          onClick={deleteSelectedEntries}
          disabled={busy || !archivePath || selected.size === 0}
        >
          <Trash2 size={20} strokeWidth={1.75} />
          {t("toolbar.delete")}
        </button>
        <div className="icon-toolbar-sep" />
        <button
          className="icon-toolbar-btn"
          onClick={() => setFindOpen((v) => !v)}
          disabled={busy || !summary}
        >
          <Search size={20} strokeWidth={1.75} />
          Find
        </button>
        <button className="icon-toolbar-btn" onClick={() => setInfoOpen(true)} disabled={busy || !summary}>
          <Info size={20} strokeWidth={1.75} />
          Info
        </button>
        <button
          className="icon-toolbar-btn"
          onClick={() => setInfoOpen(true)}
          disabled={busy || !summary?.comment}
        >
          <MessageSquare size={20} strokeWidth={1.75} />
          Comment
        </button>
        <div className="icon-toolbar-sep" />
        {busy && <span className="spinner" aria-hidden />}
        {busy && (
          <button className="cancel-btn" onClick={() => api.cancelOperation()}>
            Cancel
          </button>
        )}
      </div>

      {findOpen && (
        <div className="find-bar">
          <Search size={14} strokeWidth={1.75} />
          <input
            autoFocus
            placeholder="Find in archive…"
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
          />
          <button className="icon-btn" onClick={() => { setFindOpen(false); setFilterText(""); }}>
            <X size={14} />
          </button>
        </div>
      )}

      {busy && progress && progress.total > 0 && (
        <div className="progress-row">
          <div className="progress-bar-track">
            <div
              className="progress-bar-fill"
              style={{ width: `${Math.min(100, (progress.done / progress.total) * 100)}%` }}
            />
          </div>
          <span>{Math.min(100, Math.round((progress.done / progress.total) * 100))}%</span>
        </div>
      )}

      <div className="path-bar">{archivePath ?? t("status.noArchive")}</div>

      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th></th>
              <th>{t("table.name")}</th>
              <th>{t("table.size")}</th>
              <th>{t("table.compressed")}</th>
              <th>{t("table.ratio")}</th>
              <th>{t("table.modified")}</th>
            </tr>
          </thead>
          <tbody>
            {summary?.entries
              .filter((entry) => entry.name.toLowerCase().includes(filterText.toLowerCase()))
              .map((entry, i) => {
              const ratio = entry.size > 0 ? Math.round((1 - entry.compressed_size / entry.size) * 100) : 0;
              return (
                <tr
                  key={entry.name}
                  className={`row-in ${selected.has(entry.name) ? "selected" : ""}`}
                  style={{ animationDelay: `${Math.min(i, 25) * 12}ms` }}
                  onClick={() => toggleSelect(entry.name)}
                  onContextMenu={(event) => showContextMenu(event, entry.name)}
                >
                  <td>{entry.is_dir ? "📁" : "📄"}</td>
                  <td>{entry.name}</td>
                  <td>{formatBytes(entry.size)}</td>
                  <td>{formatBytes(entry.compressed_size)}</td>
                  <td>{entry.size > 0 ? `${ratio}%` : ""}</td>
                  <td>{entry.modified ?? ""}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <footer className="status-bar">
        {summary?.encrypted && <span className="badge">{t("status.encrypted")}</span>}
        {summary?.comment && <span className="archive-comment" title={summary.comment}>“{summary.comment}”</span>}
        <span>{status}</span>
      </footer>

      {contextMenu && (
        <ContextMenu
          position={{ x: contextMenu.x, y: contextMenu.y }}
          onClose={() => setContextMenu(null)}
          items={buildMenuItems(contextMenu)}
        />
      )}

      {dragActive && (
        <div className="drop-overlay">
          <div className="drop-overlay-card">
            <Package size={28} strokeWidth={1.5} />
            <span>Drop to add to archive</span>
          </div>
        </div>
      )}

      <AddArchiveDialog
        open={dialogSources !== null}
        defaultName={dialogSources ? `${basename(dialogSources[0]).replace(/\.[^/.]+$/, "")}.init` : ""}
        onCancel={() => setDialogSources(null)}
        onConfirm={async (result: ArchiveDialogResult) => {
          const sources = dialogSources;
          setDialogSources(null);
          if (sources) await runCreate(sources, result);
        }}
      />

      <PasswordDialog
        open={passwordRequest !== null}
        error={passwordError}
        purpose={passwordRequest?.purpose}
        onCancel={() => {
          setPasswordRequest(null);
          setPasswordError("");
        }}
        onSubmit={submitPassword}
      />

      <SimpleModal title="Archive info" open={infoOpen && !!summary} onClose={() => setInfoOpen(false)}>
        {summary && (
          <dl className="info-grid">
            <dt>Format</dt>
            <dd>{summary.format.toUpperCase()}</dd>
            <dt>Entries</dt>
            <dd>{summary.entries.length}</dd>
            <dt>Uncompressed</dt>
            <dd>{formatBytes(summary.total_uncompressed)}</dd>
            <dt>Compressed</dt>
            <dd>{formatBytes(summary.total_compressed)}</dd>
            <dt>Encrypted</dt>
            <dd>{summary.encrypted ? "Yes (AES-256)" : "No"}</dd>
            {summary.comment && (
              <>
                <dt>Comment</dt>
                <dd>{summary.comment}</dd>
              </>
            )}
          </dl>
        )}
      </SimpleModal>

      <SimpleModal title="About Syncinit" open={aboutOpen} onClose={() => setAboutOpen(false)}>
        <p style={{ margin: "0 0 8px", color: "var(--muted)" }}>
          A fast, modern archive manager for Windows — a practical WinRAR alternative.
        </p>
        <p style={{ margin: 0, fontSize: 12, color: "var(--faint)" }}>
          No telemetry, no network calls. See PRIVACY.md / TERMS.md / LICENSE in the install
          folder for the full policies.
        </p>
      </SimpleModal>
    </div>
  );
}

function SimpleModal({
  title,
  open,
  onClose,
  children,
}: {
  title: string;
  open: boolean;
  onClose: () => void;
  children: React.ReactNode;
}) {
  return (
    <Modal open={open} onClose={onClose} className="simple-modal" ariaLabel={title}>
      <div className="modal-header">
        <span>{title}</span>
        <button className="icon-btn" onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <div className="modal-body">{children}</div>
    </Modal>
  );
}
