import { invoke } from "@tauri-apps/api/core";

export type Format = "zip" | "tar" | "targz" | "tarxz" | "tarzst" | "tarbz2" | "sevenz";

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
}

export interface CreateOptions {
  destination: string;
  sources: string[];
  password?: string | null;
  level: number;
  format: Format;
}

export const api = {
  listArchive: (path: string) => invoke<ArchiveSummary>("list_archive", { path }),
  createArchive: (options: CreateOptions) => invoke<void>("create_archive", { options }),
  extractArchive: (path: string, destination: string, password?: string) =>
    invoke<number>("extract_archive", { path, destination, password: password ?? null }),
  testArchive: (path: string) => invoke<boolean>("test_archive", { path }),
  detectFormat: (path: string) => invoke<string | null>("detect_format", { path }),
};

export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
