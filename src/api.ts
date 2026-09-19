import { invoke } from "@tauri-apps/api/core";

export type Format = "init" | "zip" | "tar" | "targz" | "tarxz" | "tarzst" | "tarbz2" | "sevenz";

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
    invoke<number>("extract_archive", { path, destination, password: password ?? null }),
  testArchive: (path: string) => invoke<boolean>("test_archive", { path }),
  cancelOperation: () => invoke<void>("cancel_operation"),
  detectFormat: (path: string) => invoke<string | null>("detect_format", { path }),
};

export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
