// Lightweight "is there a newer release" check against GitHub — not a full
// silent auto-updater (that needs a signed update manifest + tauri-plugin-
// updater, which this project doesn't have set up). This just surfaces a
// dismissible notice pointing at the release page, same spirit as Xuro's
// updater but without the download/install machinery Xuro's has via its
// own signed-manifest setup.
const REPO = "Wooinxlkz/Syncinit";
const DISMISSED_KEY = "syncinit:update-dismissed";

export interface UpdateInfo {
  version: string;
  url: string;
  notes: string;
}

function versionParts(version: string): [number, number, number] | null {
  const match = /^v?(\d+)\.(\d+)\.(\d+)/.exec(version.trim());
  if (!match) return null;
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function isNewer(current: string, candidate: string): boolean {
  const a = versionParts(current);
  const b = versionParts(candidate);
  if (!a || !b) return false;
  for (let i = 0; i < 3; i++) {
    if (b[i] > a[i]) return true;
    if (b[i] < a[i]) return false;
  }
  return false;
}

/** Returns update info if GitHub has a newer release than `currentVersion`,
 *  the person hasn't already dismissed that exact version, and the request
 *  succeeds — null in every other case (including any network/API error,
 *  which is swallowed rather than surfaced, since this check is a courtesy
 *  and should never be the thing that breaks a launch). */
export async function checkForUpdate(currentVersion: string): Promise<UpdateInfo | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return null;
    const data = await res.json();
    const tag: string = data.tag_name ?? "";
    const version = tag.replace(/^v/, "");
    if (!isNewer(currentVersion, version)) return null;
    if (localStorage.getItem(DISMISSED_KEY) === version) return null;
    return {
      version,
      url: data.html_url ?? `https://github.com/${REPO}/releases/latest`,
      notes: typeof data.body === "string" ? data.body : "",
    };
  } catch {
    return null;
  }
}

export function dismissUpdate(version: string) {
  localStorage.setItem(DISMISSED_KEY, version);
}
