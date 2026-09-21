import { invoke as rawInvoke, type InvokeArgs } from "@tauri-apps/api/core";

/**
 * Wraps every Tauri command call with a timeout. This exists because the
 * reported "everything freezes until I restart the app" symptom, even
 * after fixing every main-thread-blocking command (v0.1.12) and rebuilding
 * the dialog system (v0.1.11), kept recurring — and a stuck `busy` state
 * from one hung promise that never resolves *or* rejects would look
 * exactly like that: every button stays disabled forever, nothing short
 * of a restart clears it, because the `finally` block resetting `busy`
 * never runs if the promise it's attached to never settles.
 *
 * Whatever the actual root cause turns out to be, this makes it
 * non-fatal: if a command doesn't settle within its timeout, the call
 * rejects with a clear error instead of hanging forever, so the calling
 * code's own `finally { setBusy(false) }` still runs and the UI recovers.
 * Heavy archive operations (create/extract/test/list/delete/password
 * change) get a long ceiling since a legitimately huge archive can take a
 * while; everything else — registry/diagnostic/settings calls that should
 * always be near-instant — gets a short one.
 */
const HEAVY_COMMANDS = new Set([
  "list_archive",
  "create_archive",
  "extract_archive",
  "test_archive",
  "delete_sources",
  "delete_entries",
  "change_archive_password",
]);

const HEAVY_TIMEOUT_MS = 10 * 60 * 1000; // 10 minutes — large real archives
const DEFAULT_TIMEOUT_MS = 20 * 1000; // 20 seconds — everything else

export async function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  const timeoutMs = HEAVY_COMMANDS.has(cmd) ? HEAVY_TIMEOUT_MS : DEFAULT_TIMEOUT_MS;
  let timer: ReturnType<typeof setTimeout>;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`"${cmd}" timed out after ${Math.round(timeoutMs / 1000)}s — the app should still be responsive now; try again`)),
      timeoutMs
    );
  });
  try {
    return await Promise.race([rawInvoke<T>(cmd, args), timeout]);
  } finally {
    clearTimeout(timer!);
  }
}
