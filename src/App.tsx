import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, formatBytes, type ArchiveSummary } from "./api";
import { useI18n, SUPPORTED_LOCALES } from "./i18n";
import "./App.css";

export default function App() {
  const { t, locale, setLocale } = useI18n();
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [summary, setSummary] = useState<ArchiveSummary | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);

  async function openArchive() {
    const file = await open({
      multiple: false,
      filters: [
        { name: "Archives", extensions: ["zip", "7z", "tar", "gz", "tgz", "xz", "zst", "bz2"] },
      ],
    });
    if (!file) return;
    setBusy(true);
    try {
      const path = file as string;
      const data = await api.listArchive(path);
      setArchivePath(path);
      setSummary(data);
      setSelected(new Set());
      setStatus(
        t("status.entries", {
          count: data.entries.length,
          size: formatBytes(data.total_uncompressed),
        })
      );
    } catch (err) {
      setStatus(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function addFilesAsNewArchive() {
    const files = await open({ multiple: true });
    if (!files) return;
    const sources = Array.isArray(files) ? files : [files];
    const destination = await save({
      defaultPath: "archive.zip",
      filters: [{ name: "Zip", extensions: ["zip"] }],
    });
    if (!destination) return;

    setBusy(true);
    try {
      await api.createArchive({ destination, sources, level: 6, format: "zip", password: null });
      const data = await api.listArchive(destination);
      setArchivePath(destination);
      setSummary(data);
      setStatus(t("status.entries", { count: data.entries.length, size: formatBytes(data.total_uncompressed) }));
    } catch (err) {
      setStatus(String(err));
    } finally {
      setBusy(false);
    }
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
        <select
          className="locale-select"
          value={locale}
          onChange={(e) => setLocale(e.target.value)}
        >
          {SUPPORTED_LOCALES.map((l) => (
            <option key={l.code} value={l.code} disabled={!l.ready}>
              {l.label}
              {!l.ready ? " (soon)" : ""}
            </option>
          ))}
        </select>
      </header>

      <div className="toolbar">
        <button onClick={addFilesAsNewArchive} disabled={busy}>
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
            {summary?.entries.map((entry) => {
              const ratio =
                entry.size > 0 ? Math.round((1 - entry.compressed_size / entry.size) * 100) : 0;
              return (
                <tr
                  key={entry.name}
                  className={selected.has(entry.name) ? "selected" : ""}
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
    </div>
  );
}
