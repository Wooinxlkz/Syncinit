import { useEffect, useState, type MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { confirm, open } from "@tauri-apps/plugin-dialog";
// (no plugin-shell import — reveal_in_file_manager on the Rust side avoids
// the console-flash the shell plugin's open() causes on Windows)
import { api, formatBytes, type ArchiveSummary } from "./api";
import { useI18n } from "./i18n";
import { LocaleDropdown } from "./LocaleDropdown";
import AddArchiveDialog, { type ArchiveDialogResult } from "./AddArchiveDialog";
import PasswordDialog from "./PasswordDialog";
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
  return format === "arc"
    ? ".arc"
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

export default function App() {
  const { t, locale, setLocale } = useI18n();
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [summary, setSummary] = useState<ArchiveSummary | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [dialogSources, setDialogSources] = useState<string[] | null>(null);
  const [passwordRequest, setPasswordRequest] = useState<PasswordRequest | null>(null);
  const [passwordError, setPasswordError] = useState("");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  // Pick up context-menu launches: "Add to archive...", "Add to X.arc",
  // "Compress and email...", "Extract Here", or a double-clicked archive.
  useEffect(() => {
    // Register in HKCU as well as the installer HKCR entries. This makes the
    // Explorer menu work when running a development build or an unpacked exe.
    invoke<boolean>("register_context_menu").catch((err) =>
      console.warn("Context menu registration failed:", err)
    );
    const closeMenu = () => setContextMenu(null);
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setContextMenu(null);
    };
    window.addEventListener("click", closeMenu);
    window.addEventListener("keydown", closeOnEscape);

    invoke<LaunchAction>("get_launch_action").then(async (action) => {
      if (!action || action.mode === "none") return;
      try {
        if (action.mode === "add-dialog") {
          setDialogSources(action.paths);
        } else if (action.mode === "add-default") {
          await runCreate(action.paths, {
            name: `${basename(action.paths[0])}.arc`,
            format: "arc",
            level: 6,
            password: "",
            deleteAfter: false,
            testAfter: false,
            comment: "",
            smartStore: true,
          });
        } else if (action.mode === "add-and-mail") {
          const dest = await runCreate(action.paths, {
            name: `${basename(action.paths[0])}.arc`,
            format: "arc",
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
    });
    return () => {
      window.removeEventListener("click", closeMenu);
      window.removeEventListener("keydown", closeOnEscape);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
      filters: [{ name: "Archives", extensions: ["arc", "zip", "7z", "tar", "gz", "tgz", "xz", "zst", "bz2"] }],
    });
    if (file) await loadArchive(file as string);
  }

  async function runCreate(sources: string[], result: ArchiveDialogResult): Promise<string | null> {
    let destination = normalizeDestination(result.name.trim(), result.format);
    if (!destination.includes("/") && !destination.includes("\\")) {
      destination = `${dirname(sources[0])}/${destination}`;
    }
    setBusy(true);
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
          { title: "Tugur — delete originals", kind: "warning" },
        );
        if (approved) {
          await api.deleteSources(sources, destination);
        }
      }
      await loadArchive(destination);
      return destination;
    } catch (err) {
      setStatus(String(err));
      return null;
    } finally {
      setBusy(false);
    }
  }

  async function addFilesViaDialog() {
    const files = await open({ multiple: true });
    if (!files) return;
    setDialogSources(Array.isArray(files) ? files : [files]);
  }

  async function extractTo(source: string, destination: string, password?: string) {
    setBusy(true);
    try {
      const count = await api.extractArchive(source, destination, password);
      setPasswordRequest(null);
      setPasswordError("");
      setStatus(t("toast.extracted", { count, dest: destination }));
    } catch (err) {
      if (isPasswordError(err)) {
        setPasswordRequest({ path: source, purpose: "extract", destination });
        setPasswordError(password ? "Incorrect password. Try again." : "");
      } else {
        setStatus(String(err));
      }
    } finally {
      setBusy(false);
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

  return (
    <div className="app" onContextMenu={showContextMenu}>
      <header className="titlebar">
        <span className="app-name">{t("app.title")}</span>
        <LocaleDropdown locale={locale} onChange={setLocale} />
      </header>

      <div className="toolbar">
        <button onClick={addFilesViaDialog} disabled={busy}>
          {t("toolbar.add")}
        </button>
        <button onClick={openArchive} disabled={busy}>
          Open
        </button>
        <button onClick={extractCurrent} disabled={busy || !archivePath}>
          {t("toolbar.extract")}
        </button>
        <button onClick={testCurrent} disabled={busy || !archivePath}>
          {t("toolbar.test")}
        </button>
        <button disabled={busy || selected.size === 0}>{t("toolbar.delete")}</button>
        {busy && <span className="spinner" aria-hidden />}
      </div>

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
            {summary?.entries.map((entry, i) => {
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
        <div
          className="context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onClick={(event) => event.stopPropagation()}
          onContextMenu={(event) => event.preventDefault()}
        >
          {contextMenu.entry && (
            <button onClick={() => { toggleSelect(contextMenu.entry!); setContextMenu(null); }}>
              {selected.has(contextMenu.entry) ? "Unselect entry" : "Select entry"}
            </button>
          )}
          <button onClick={addFilesViaDialog}>Add files to archive…</button>
          <button onClick={openArchive}>Open archive…</button>
          <button disabled={!archivePath} onClick={extractCurrent}>Extract to…</button>
          <button disabled={!archivePath} onClick={testCurrent}>Test archive</button>
          <button disabled={!summary} onClick={selectAllEntries}>Select all entries</button>
          {selected.size > 0 && (
            <button onClick={() => { setSelected(new Set()); setContextMenu(null); }}>
              Clear selection
            </button>
          )}
        </div>
      )}

      {dialogSources && (
        <AddArchiveDialog
           defaultName={`${basename(dialogSources[0]).replace(/\.[^/.]+$/, "")}.arc`}
          onCancel={() => setDialogSources(null)}
          onConfirm={async (result: ArchiveDialogResult) => {
            const sources = dialogSources;
            setDialogSources(null);
            await runCreate(sources, result);
          }}
        />
      )}

      {passwordRequest && (
        <PasswordDialog
          error={passwordError}
          purpose={passwordRequest.purpose}
          onCancel={() => {
            setPasswordRequest(null);
            setPasswordError("");
          }}
          onSubmit={submitPassword}
        />
      )}
    </div>
  );
}
