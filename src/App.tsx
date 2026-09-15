import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { open as openPath } from "@tauri-apps/plugin-shell";
import { api, formatBytes, type ArchiveSummary } from "./api";
import { useI18n, SUPPORTED_LOCALES } from "./i18n";
import AddArchiveDialog, { type ArchiveDialogResult } from "./AddArchiveDialog";
import "./App.css";

type LaunchAction =
  | { mode: "add-dialog"; paths: string[] }
  | { mode: "add-default"; paths: string[] }
  | { mode: "add-and-mail"; paths: string[] }
  | { mode: "extract-here"; paths: string[] }
  | { mode: "open-archive"; path: string }
  | { mode: "none" };

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

export default function App() {
  const { t, locale, setLocale } = useI18n();
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [summary, setSummary] = useState<ArchiveSummary | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [dialogSources, setDialogSources] = useState<string[] | null>(null);

  // Pick up context-menu launches: "Add to archive...", "Add to X.zip",
  // "Compress and email...", "Extract Here", or a double-clicked archive.
  useEffect(() => {
    invoke<LaunchAction>("get_launch_action").then(async (action) => {
      if (!action || action.mode === "none") return;
      try {
        if (action.mode === "add-dialog") {
          setDialogSources(action.paths);
        } else if (action.mode === "add-default") {
          await runCreate(action.paths, {
            name: `${basename(action.paths[0])}.zip`,
            format: "zip",
            level: 6,
            password: "",
            deleteAfter: false,
            testAfter: false,
            comment: "",
          });
        } else if (action.mode === "add-and-mail") {
          const dest = await runCreate(action.paths, {
            name: `${basename(action.paths[0])}.zip`,
            format: "zip",
            level: 6,
            password: "",
            deleteAfter: false,
            testAfter: false,
            comment: "",
          });
          if (dest) await openPath(dirname(dest));
        } else if (action.mode === "extract-here") {
          const src = action.paths[0];
          const count = await api.extractArchive(src, dirname(src));
          setStatus(t("toast.extracted", { count, dest: dirname(src) }));
        } else if (action.mode === "open-archive") {
          await loadArchive(action.path);
        }
      } catch (err) {
        setStatus(String(err));
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function loadArchive(path: string) {
    setBusy(true);
    try {
      const data = await api.listArchive(path);
      setArchivePath(path);
      setSummary(data);
      setSelected(new Set());
      setStatus(
        t("status.entries", { count: data.entries.length, size: formatBytes(data.total_uncompressed) })
      );
    } catch (err) {
      setStatus(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function openArchive() {
    const file = await open({
      multiple: false,
      filters: [{ name: "Archives", extensions: ["zip", "7z", "tar", "gz", "tgz", "xz", "zst", "bz2"] }],
    });
    if (file) await loadArchive(file as string);
  }

  async function runCreate(sources: string[], result: ArchiveDialogResult): Promise<string | null> {
    let destination = result.name;
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
      });
      if (result.testAfter) await api.testArchive(destination);
      if (result.deleteAfter) {
        // Deleting originals is destructive — left for the user to confirm
        // manually in v0.1.1 rather than silently removing files.
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

  async function extractCurrent() {
    if (!archivePath) return;
    const destination = await open({ directory: true });
    if (!destination) return;
    setBusy(true);
    try {
      const count = await api.extractArchive(archivePath, destination as string);
      setStatus(t("toast.extracted", { count, dest: destination as string }));
    } catch (err) {
      setStatus(String(err));
    } finally {
      setBusy(false);
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

  return (
    <div className="app">
      <header className="titlebar">
        <span className="app-name">{t("app.title")}</span>
        <select className="locale-select" value={locale} onChange={(e) => setLocale(e.target.value)}>
          {SUPPORTED_LOCALES.map((l) => (
            <option key={l.code} value={l.code} disabled={!l.ready}>
              {l.label}
              {!l.ready ? " (soon)" : ""}
            </option>
          ))}
        </select>
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
        <span>{status}</span>
      </footer>

      {dialogSources && (
        <AddArchiveDialog
          defaultName={`${basename(dialogSources[0]).replace(/\.[^/.]+$/, "")}.zip`}
          onCancel={() => setDialogSources(null)}
          onConfirm={async (result: ArchiveDialogResult) => {
            const sources = dialogSources;
            setDialogSources(null);
            await runCreate(sources, result);
          }}
        />
      )}
    </div>
  );
}
