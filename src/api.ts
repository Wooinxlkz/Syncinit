import { invoke } from "./invokeSafe";

export type Format = "init" | "zip" | "tar" | "targz" | "tarxz" | "tarzst" | "tarbz2" | "sevenz" | "rar";

export interface ArchiveEntry {
  name: string;
  is_dir: boolean;
  size: number;
  compressed_size: number;
  modified?: string | null;
  crc32?: number | null;
}

export interface ArchiveSummary {
  format: Format;
  entries: ArchiveEntry[];
  total_uncompressed: number;
  total_compressed: number;
  encrypted: boolean;
  comment?: string | null;
}

export interface CreateOptions {
  destination: string;
  sources: string[];
  password?: string | null;
  level: number;
  format: Format;
  smart_store: boolean;
  comment?: string | null;
}

export const api = {
  listArchive: (path: string, password?: string) =>
    invoke<ArchiveSummary>("list_archive", { path, password: password ?? null }),
  // Returns paths that couldn't be added (permission denied, a broken
  // symlink, a Windows long-path failure, etc.) instead of silently
  // leaving them out — an empty array means everything made it in.
  createArchive: (options: CreateOptions) => invoke<string[]>("create_archive", { options }),
  deleteSources: (sources: string[], destination: string) =>
    invoke<number>("delete_sources", { sources, destination }),
  deleteEntries: (path: string, names: string[]) => invoke<number>("delete_entries", { path, names }),
  extractArchive: (path: string, destination: string, password?: string) =>
    invoke<{ count: number; warnings: string[] }>("extract_archive", {
      path,
      destination,
      password: password ?? null,
    }),
  testArchive: (path: string) => invoke<boolean>("test_archive", { path }),
  // old_password: current password if the archive is already protected
  // (omit/undefined if it isn't). new_password: empty string or omitted
  // removes protection entirely.
  changeArchivePassword: (path: string, oldPassword: string | undefined, newPassword: string | undefined) =>
    invoke<void>("change_archive_password", {
      path,
      oldPassword: oldPassword || null,
      newPassword: newPassword || null,
    }),
  cancelOperation: () => invoke<void>("cancel_operation"),
  detectFormat: (path: string) => invoke<string | null>("detect_format", { path }),
  // OS credential store (Windows Credential Manager) — opt-in via the
  // "Remember this password" checkbox on the unlock dialog. Syncinit
  // never writes the password itself to disk; Windows' own store does.
  savePassword: (path: string, password: string) => invoke<void>("save_archive_password", { path, password }),
  getSavedPassword: (path: string) => invoke<string | null>("get_saved_archive_password", { path }),
  forgetPassword: (path: string) => invoke<void>("forget_archive_password", { path }),
};

export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
