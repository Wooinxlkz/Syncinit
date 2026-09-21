import { useEffect, useState } from "react";
import { Copy, Check, RefreshCw, Wrench, Trash2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { version as APP_VERSION } from "../package.json";

/**
 * Everything needed to actually diagnose "it's not working" reports without
 * needing devtools open: raw launch argv, what it parsed into, every
 * context-menu registration/relaunch event since startup, current registry
 * state, and the .init icon's actual on-disk status. One button copies it
 * all as plain text to paste back for debugging. Also: explicit Reinstall /
 * Uninstall for the context menu — matching how e.g. Ziplark ships
 * `shell-integration install/status/uninstall` as user-triggered actions
 * instead of silent, invisible auto-registration on every launch.
 */
export function Diagnostics() {
  const [log, setLog] = useState<string[]>([]);
  const [registry, setRegistry] = useState<unknown>(null);
  const [icon, setIcon] = useState<unknown>(null);
  const [rawArgs, setRawArgs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState(false);
  const [actionMsg, setActionMsg] = useState("");
  const [acting, setActing] = useState(false);

  async function refresh() {
    setLoading(true);
    const [logRes, registryRes, iconRes, argsRes] = await Promise.allSettled([
      invoke<string[]>("get_diagnostics"),
      invoke("debug_context_menu"),
      invoke("get_icon_diagnostics"),
      invoke<string[]>("get_raw_args"),
    ]);
    setLog(logRes.status === "fulfilled" ? logRes.value : [`error: ${logRes.reason}`]);
    setRegistry(registryRes.status === "fulfilled" ? registryRes.value : { error: String(registryRes.reason) });
    setIcon(iconRes.status === "fulfilled" ? iconRes.value : { error: String(iconRes.reason) });
    setRawArgs(argsRes.status === "fulfilled" ? argsRes.value : [`error: ${argsRes.reason}`]);
    setLoading(false);
  }

  useEffect(() => {
    refresh();
  }, []);

  const asText = () =>
    [
      `Syncinit v${APP_VERSION} diagnostics`,
      "",
      "-- current process argv --",
      JSON.stringify(rawArgs, null, 2),
      "",
      "-- .init icon status --",
      JSON.stringify(icon, null, 2),
      "",
      "-- context menu registry state --",
      JSON.stringify(registry, null, 2),
      "",
      "-- event log (startup + relaunches + registration outcomes) --",
      ...log,
    ].join("\n");

  async function copyAll() {
    try {
      await navigator.clipboard.writeText(asText());
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard permission denied or unavailable — the text is still
      // fully visible and selectable in the block below as a fallback.
    }
  }

  async function reinstall() {
    setActing(true);
    setActionMsg("");
    try {
      await invoke("reinstall_context_menu");
      setActionMsg("Reinstalled.");
      await refresh();
    } catch (err) {
      setActionMsg(`Reinstall failed: ${err}`);
    } finally {
      setActing(false);
    }
  }

  async function uninstall() {
    setActing(true);
    setActionMsg("");
    try {
      await invoke("uninstall_context_menu");
      setActionMsg("Uninstalled — right-click entries removed. Reinstall to bring them back.");
      await refresh();
    } catch (err) {
      setActionMsg(`Uninstall failed: ${err}`);
    } finally {
      setActing(false);
    }
  }

  return (
    <section>
      <div className="diag-toolbar">
        <button className="btn-secondary" onClick={refresh} disabled={loading}>
          <RefreshCw size={13} strokeWidth={2} className={loading ? "spin" : undefined} />
          Refresh
        </button>
        <button className="btn-secondary" onClick={copyAll}>
          {copied ? <Check size={13} strokeWidth={2} /> : <Copy size={13} strokeWidth={2} />}
          {copied ? "Copied" : "Copy all"}
        </button>
        <button className="btn-secondary" onClick={reinstall} disabled={acting}>
          <Wrench size={13} strokeWidth={2} />
          Reinstall context menu
        </button>
        <button className="btn-secondary" onClick={uninstall} disabled={acting}>
          <Trash2 size={13} strokeWidth={2} />
          Uninstall context menu
        </button>
      </div>
      {actionMsg && <p className="diag-action-msg">{actionMsg}</p>}
      <pre className="diag-block">{asText()}</pre>
    </section>
  );
}
