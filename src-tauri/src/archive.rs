use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("7z error: {0}")]
    SevenZ(String),
    #[error("unsupported or unrecognized archive format: {0}")]
    UnsupportedFormat(String),
    #[error("archive is password protected")]
    PasswordRequired,
    #[error("incorrect password")]
    BadPassword,
    #[error("integrity check failed for entry: {0}")]
    IntegrityFailed(String),
    #[error("refusing to delete a source that contains the output archive")]
    UnsafeDelete,
    #[error("cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, ArchiveError>;

/// Progress + cancellation, threaded through every create/extract call.
/// `on_progress(done_bytes, total_bytes)` fires periodically; `cancelled`
/// is polled between chunks and turns into `ArchiveError::Cancelled` the
/// moment it flips true, so "Cancel" in the UI stops the operation instead
/// of just hiding a spinner that keeps running underneath.
pub struct ProgressCtx<'a> {
    pub on_progress: Option<&'a dyn Fn(u64, u64)>,
    pub cancelled: Option<&'a AtomicBool>,
}

impl<'a> ProgressCtx<'a> {
    pub fn none() -> Self {
        Self { on_progress: None, cancelled: None }
    }
    fn tick(&self, done: u64, total: u64) -> Result<()> {
        if let Some(cb) = self.on_progress {
            cb(done, total);
        }
        if self.cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(ArchiveError::Cancelled);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Init,
    Zip,
    Tar,
    TarGz,
    TarXz,
    TarZst,
    TarBz2,
    SevenZ,
}

impl Format {
    pub fn from_path(path: &Path) -> Option<Format> {
        let name = path.file_name()?.to_string_lossy().to_lowercase();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Some(Format::TarGz)
        } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
            Some(Format::TarXz)
        } else if name.ends_with(".tar.zst") {
            Some(Format::TarZst)
        } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
            Some(Format::TarBz2)
        } else if name.ends_with(".tar") {
            Some(Format::Tar)
        } else if name.ends_with(".7z") {
            Some(Format::SevenZ)
        } else if name.ends_with(".init") {
            Some(Format::Init)
        } else if name.ends_with(".zip") {
            Some(Format::Zip)
        } else {
            None
        }
    }

    /// Formats Syncinit can write natively. Other formats are read/extract-only.
    pub fn is_writable(&self) -> bool {
        matches!(
            self,
            Format::Init | Format::Zip | Format::Tar | Format::TarGz | Format::TarZst
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: u64,
    pub modified: Option<String>,
    pub crc32: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveSummary {
    pub format: Format,
    pub entries: Vec<ArchiveEntry>,
    pub total_uncompressed: u64,
    pub total_compressed: u64,
    pub encrypted: bool,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOptions {
    pub destination: String,
    pub sources: Vec<String>,
    pub password: Option<String>,
    /// 0 = store, 1..=9 mapped to deflate/zstd level depending on format
    pub level: u8,
    pub format: Format,
    /// Store formats that are already compressed instead of wasting CPU on them.
    #[serde(default = "default_true")]
    pub smart_store: bool,
    #[serde(default)]
    pub comment: Option<String>,
}

fn default_true() -> bool {
    true
}

/// List the contents of an archive without extracting it.
pub fn list_archive(path: &str, password: Option<&str>) -> Result<ArchiveSummary> {
    let path = Path::new(path);
    let format = Format::from_path(path)
        .ok_or_else(|| ArchiveError::UnsupportedFormat(path.display().to_string()))?;

    match format {
        Format::Init => list_zip(path, Format::Init, password),
        Format::Zip => list_zip(path, Format::Zip, password),
        Format::Tar | Format::TarGz | Format::TarXz | Format::TarZst | Format::TarBz2 => {
            list_tar(path, format)
        }
        Format::SevenZ => list_7z(path),
    }
}

fn list_zip(path: &Path, format: Format, password: Option<&str>) -> Result<ArchiveSummary> {
    verify_zip_password(path, password)?;
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
    let mut entries = Vec::with_capacity(zip.len());
    let mut total_uncompressed = 0u64;
    let mut total_compressed = 0u64;
    let mut encrypted = false;

    for i in 0..zip.len() {
        let entry = zip.by_index_raw(i)?;
        if entry.encrypted() {
            encrypted = true;
        }
        total_uncompressed += entry.size();
        total_compressed += entry.compressed_size();
        entries.push(ArchiveEntry {
            name: entry.name().to_string(),
            is_dir: entry.is_dir(),
            size: entry.size(),
            compressed_size: entry.compressed_size(),
            modified: entry.last_modified().map(|d| {
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}",
                    d.year(),
                    d.month(),
                    d.day(),
                    d.hour(),
                    d.minute()
                )
            }),
            crc32: Some(entry.crc32()),
        });
    }

    Ok(ArchiveSummary {
        format,
        entries,
        total_uncompressed,
        total_compressed,
        encrypted,
        comment: if zip.comment().is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(zip.comment()).into_owned())
        },
    })
}

/// Listing raw ZIP metadata does not authenticate an encrypted archive. Do a
/// complete authenticated read before showing its contents so opening an
/// encrypted .init/.zip always requires the password.
fn verify_zip_password(path: &Path, password: Option<&str>) -> Result<()> {
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
    let mut encrypted = false;
    for i in 0..zip.len() {
        let raw = zip.by_index_raw(i)?;
        if raw.encrypted() {
            encrypted = true;
        }
    }
    if !encrypted {
        return Ok(());
    }

    let password = password.ok_or(ArchiveError::PasswordRequired)?;
    for i in 0..zip.len() {
        let encrypted = zip.by_index_raw(i)?.encrypted();
        if !encrypted {
            continue;
        }
        let mut entry = zip
            .by_index_decrypt(i, password.as_bytes())
            .map_err(|_| ArchiveError::BadPassword)?;
        std::io::copy(&mut entry, &mut std::io::sink()).map_err(|_| ArchiveError::BadPassword)?;
    }
    Ok(())
}

fn list_tar(path: &Path, format: Format) -> Result<ArchiveSummary> {
    let file = File::open(path)?;
    let reader: Box<dyn Read> = match format {
        Format::TarGz => Box::new(flate2::read::GzDecoder::new(file)),
        Format::TarXz => Box::new(xz2::read::XzDecoder::new(file)),
        Format::TarZst => Box::new(zstd::stream::Decoder::new(file)?),
        Format::TarBz2 => Box::new(bzip2::read::BzDecoder::new(file)),
        Format::Tar => Box::new(file),
        _ => unreachable!(),
    };
    let mut archive = tar::Archive::new(reader);
    let mut entries = Vec::new();
    let mut total_uncompressed = 0u64;

    for entry in archive.entries()? {
        let entry = entry?;
        let header = entry.header();
        let size = header.size().unwrap_or(0);
        total_uncompressed += size;
        entries.push(ArchiveEntry {
            name: entry.path()?.to_string_lossy().to_string(),
            is_dir: header.entry_type().is_dir(),
            size,
            compressed_size: 0, // tar entries aren't individually compressed
            modified: header.mtime().ok().map(|t| t.to_string()),
            crc32: None,
        });
    }

    Ok(ArchiveSummary {
        format,
        entries,
        total_uncompressed,
        total_compressed: std::fs::metadata(path)?.len(),
        encrypted: false,
        comment: None,
    })
}

fn list_7z(path: &Path) -> Result<ArchiveSummary> {
    let archive =
        sevenz_rust::Archive::open(path).map_err(|e| ArchiveError::SevenZ(e.to_string()))?;
    let mut entries = Vec::new();
    let mut total_uncompressed = 0u64;

    for entry in archive.files.iter() {
        total_uncompressed += entry.size();
        entries.push(ArchiveEntry {
            name: entry.name().to_string(),
            is_dir: entry.is_directory(),
            size: entry.size(),
            compressed_size: 0,
            modified: None,
            crc32: if entry.has_crc {
                Some(entry.crc as u32)
            } else {
                None
            },
        });
    }

    Ok(ArchiveSummary {
        format: Format::SevenZ,
        entries,
        total_uncompressed,
        total_compressed: std::fs::metadata(path)?.len(),
        encrypted: archive.folders.iter().any(|folder| {
            folder.coders.iter().any(|coder| {
                coder.decompression_method_id() == sevenz_rust::SevenZMethod::ID_AES256SHA256
            })
        }),
        comment: None,
    })
}

/// Create a new archive from a set of source files/directories. Returns
/// paths that couldn't be added (permission denied, a path too long for
/// Windows' 260-character limit, a broken symlink, etc.) instead of
/// silently omitting them from the archive — an archiver that quietly
/// leaves things out and still reports success is worse than one that
/// tells you what it couldn't get to.
pub fn create_archive(opts: &CreateOptions, ctx: &ProgressCtx) -> Result<Vec<String>> {
    if !opts.format.is_writable() {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "{:?} writing not implemented yet",
            opts.format
        )));
    }

    match opts.format {
        Format::Init | Format::Zip => create_zip(opts, ctx),
        Format::Tar => create_tar(opts, None, ctx),
        Format::TarGz => create_tar(opts, Some(CompressorKind::Gzip), ctx),
        Format::TarZst => create_tar(opts, Some(CompressorKind::Zstd), ctx),
        _ => unreachable!(),
    }
}

enum CompressorKind {
    Gzip,
    Zstd,
}

/// Total bytes across all sources, for progress denominators. Cheap
/// metadata-only walk, no file contents touched. `follow_links(true)` so a
/// symlinked directory's actual contents get counted (and, in
/// create_zip_into/create_tar, actually walked and added) instead of the
/// symlink just appearing as an empty folder in the archive.
fn estimate_total_bytes(sources: &[String]) -> u64 {
    let mut total = 0u64;
    for src in sources {
        let src_path = PathBuf::from(src);
        if src_path.is_dir() {
            for entry in walkdir::WalkDir::new(&src_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        } else if let Ok(meta) = std::fs::metadata(&src_path) {
            total += meta.len();
        }
    }
    total.max(1)
}

/// `io::copy`, but reporting cumulative progress and checking for
/// cancellation every 64 KiB instead of only at file boundaries — so a
/// single large file still gives responsive progress/cancel, not just
/// many small ones.
fn copy_with_progress<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    done_so_far: u64,
    total: u64,
    ctx: &ProgressCtx,
) -> Result<u64> {
    let mut buf = [0u8; 64 * 1024];
    let mut copied = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        copied += n as u64;
        ctx.tick(done_so_far + copied, total)?;
    }
    Ok(copied)
}

fn create_zip(opts: &CreateOptions, ctx: &ProgressCtx) -> Result<Vec<String>> {
    let destination = Path::new(&opts.destination);
    let temp_path = destination.with_extension(format!(
        "{}.syncinit-part-{}",
        destination
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("init"),
        std::process::id()
    ));

    let result = create_zip_into(opts, ctx, destination, &temp_path);
    if result.is_err() {
        // Cancelled or failed partway through — don't leave a half-written
        // temp file sitting next to the destination.
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

fn create_zip_into(
    opts: &CreateOptions,
    ctx: &ProgressCtx,
    destination: &Path,
    temp_path: &Path,
) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    let total = estimate_total_bytes(&opts.sources);
    let mut done = 0u64;

    let file = BufWriter::new(File::create(temp_path)?);
    let mut writer = zip::ZipWriter::new(file);

    if let Some(comment) = opts
        .comment
        .as_deref()
        .filter(|comment| !comment.trim().is_empty())
    {
        writer.set_comment(comment.to_owned());
    }

    for src in &opts.sources {
        let src_path = PathBuf::from(src);
        let base_name = src_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if src_path.is_dir() {
            for item in walkdir::WalkDir::new(&src_path).follow_links(true).into_iter() {
                let entry = match item {
                    Ok(e) => e,
                    Err(e) => {
                        // Previously: `.filter_map(|e| e.ok())` here just
                        // dropped this path with zero trace — the archive
                        // would finish and report success even though a
                        // file was missing from it. Record why instead and
                        // keep going with everything that *is* readable.
                        warnings.push(format!(
                            "{}: {}",
                            e.path().map(|p| p.display().to_string()).unwrap_or_default(),
                            e
                        ));
                        continue;
                    }
                };
                let path = entry.path();
                let relative = path.strip_prefix(&src_path).unwrap_or(path);
                let zip_path = if relative.as_os_str().is_empty() {
                    format!("{base_name}/")
                } else {
                    format!(
                        "{base_name}/{}",
                        relative.to_string_lossy().replace('\\', "/")
                    )
                };

                if path.is_dir() {
                    writer.add_directory(zip_path, zip_options(opts, false))?;
                } else {
                    // Open *before* start_file — calling start_file first and
                    // then failing to open would leave a dangling empty entry
                    // in the zip instead of just cleanly not adding it.
                    let mut f = match File::open(path) {
                        Ok(f) => BufReader::with_capacity(128 * 1024, f),
                        Err(e) => {
                            warnings.push(format!("{}: {}", path.display(), e));
                            continue;
                        }
                    };
                    writer.start_file(zip_path, zip_options(opts, should_store(path, opts)))?;
                    done += copy_with_progress(&mut f, &mut writer, done, total, ctx)?;
                }
            }
        } else {
            writer.start_file(base_name, zip_options(opts, should_store(&src_path, opts)))?;
            let mut f = BufReader::with_capacity(128 * 1024, File::open(&src_path)?);
            done += copy_with_progress(&mut f, &mut writer, done, total, ctx)?;
        }
    }

    writer
        .finish()?
        .into_inner()
        .map_err(|e| e.into_error())?
        .flush()?;
    if destination.exists() {
        std::fs::remove_file(destination)?;
    }
    std::fs::rename(temp_path, destination)?;
    Ok(warnings)
}

fn should_store(path: &Path, opts: &CreateOptions) -> bool {
    if opts.level == 0 || !opts.smart_store {
        return opts.level == 0;
    }
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some(
            "7z" | "init"
                | "avi"
                | "bz2"
                | "gz"
                | "gif"
                | "heic"
                | "jpeg"
                | "jpg"
                | "m4a"
                | "m4v"
                | "mkv"
                | "mov"
                | "mp3"
                | "mp4"
                | "ogg"
                | "pdf"
                | "png"
                | "rar"
                | "webm"
                | "webp"
                | "xz"
                | "zip"
                | "zst"
        )
    )
}

fn zip_options(opts: &CreateOptions, store: bool) -> zip::write::FileOptions<()> {
    let method = if store {
        zip::CompressionMethod::Stored
    } else {
        zip::CompressionMethod::Deflated
    };
    let mut options = zip::write::FileOptions::default()
        .compression_method(method)
        .compression_level(if store {
            None
        } else {
            Some(opts.level.min(9) as i64)
        });
    if let Some(password) = &opts.password {
        options = options.with_aes_encryption(zip::AesMode::Aes256, password);
    }
    options
}

fn create_tar(opts: &CreateOptions, compressor: Option<CompressorKind>, ctx: &ProgressCtx) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    let total = estimate_total_bytes(&opts.sources);
    let mut done = 0u64;

    let file = File::create(&opts.destination)?;
    let writer: Box<dyn Write> = match compressor {
        Some(CompressorKind::Gzip) => Box::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::new(opts.level.min(9) as u32),
        )),
        Some(CompressorKind::Zstd) => {
            let mut encoder = zstd::stream::Encoder::new(file, opts.level.min(19) as i32)?;
            // Real multi-threaded compression, not a rayon-over-the-top
            // approximation: libzstd's own worker-thread pool. Falls back
            // to single-threaded silently if this build of zstd wasn't
            // compiled with multithread support (the `zstdmt` feature) —
            // still correct, just not faster, so this is safe either way.
            let _ = encoder.multithread(num_cpus());
            Box::new(encoder.auto_finish())
        }
        None => Box::new(file),
    };
    let mut tar_builder = tar::Builder::new(writer);

    for src in &opts.sources {
        let src_path = PathBuf::from(src);
        let base_name = src_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if src_path.is_dir() {
            for item in walkdir::WalkDir::new(&src_path).follow_links(true).into_iter() {
                let entry = match item {
                    Ok(e) => e,
                    Err(e) => {
                        warnings.push(format!(
                            "{}: {}",
                            e.path().map(|p| p.display().to_string()).unwrap_or_default(),
                            e
                        ));
                        continue;
                    }
                };
                let path = entry.path();
                let relative = path.strip_prefix(&src_path).unwrap_or(path);
                let tar_path = if relative.as_os_str().is_empty() {
                    format!("{base_name}/")
                } else {
                    format!("{base_name}/{}", relative.to_string_lossy().replace('\\', "/"))
                };

                if path.is_dir() {
                    // Preserve empty directories too, same as the zip path's
                    // add_directory call.
                    tar_builder.append_dir(&tar_path, path)?;
                    continue;
                }

                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let mut f = match File::open(path) {
                    Ok(f) => BufReader::with_capacity(128 * 1024, f),
                    Err(e) => {
                        warnings.push(format!("{}: {}", path.display(), e));
                        continue;
                    }
                };
                let mut header = tar::Header::new_gnu();
                header.set_size(size);
                header.set_mode(0o644);
                header.set_cksum();
                // tar::Builder needs the exact byte count up front (it's in
                // the header), so we can't stream progress through
                // append_data's own writer the way the zip path does —
                // tick once per file instead of mid-file.
                tar_builder.append_data(&mut header, &tar_path, &mut f)?;
                done += size;
                ctx.tick(done, total)?;
            }
        } else {
            let mut f = File::open(&src_path)?;
            let size = f.metadata().map(|m| m.len()).unwrap_or(0);
            tar_builder.append_file(&base_name, &mut f)?;
            done += size;
            ctx.tick(done, total)?;
        }
    }

    tar_builder.finish()?;
    Ok(warnings)
}

fn num_cpus() -> u32 {
    std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1)
}

/// Extract an archive to a destination directory. Supports zip, tar variants,
/// and 7z (read-only). Returns the number of entries extracted.
pub fn extract_archive(path: &str, destination: &str, password: Option<&str>, ctx: &ProgressCtx) -> Result<usize> {
    let path = Path::new(path);
    let dest = Path::new(destination);
    std::fs::create_dir_all(dest)?;

    let format = Format::from_path(path)
        .ok_or_else(|| ArchiveError::UnsupportedFormat(path.display().to_string()))?;

    match format {
        Format::Init | Format::Zip => extract_zip(path, dest, password, ctx),
        Format::Tar | Format::TarGz | Format::TarXz | Format::TarZst | Format::TarBz2 => {
            extract_tar(path, dest, format, ctx)
        }
        Format::SevenZ => extract_7z(path, dest, password),
    }
}

fn extract_zip(path: &Path, dest: &Path, password: Option<&str>, ctx: &ProgressCtx) -> Result<usize> {
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
    let mut count = 0;
    let total: u64 = (0..zip.len())
        .filter_map(|i| zip.by_index_raw(i).ok().map(|e| e.size()))
        .sum::<u64>()
        .max(1);
    let mut done = 0u64;

    for i in 0..zip.len() {
        let mut entry = if let Some(pw) = password {
            match zip.by_index_decrypt(i, pw.as_bytes()) {
                Ok(e) => e,
                Err(_) => return Err(ArchiveError::BadPassword),
            }
        } else {
            let raw = zip.by_index_raw(i)?;
            if raw.encrypted() {
                return Err(ArchiveError::PasswordRequired);
            }
            drop(raw);
            zip.by_index(i)?
        };

        let out_path = safe_archive_path(dest, entry.name())?;
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = File::create(&out_path)?;
            done += copy_with_progress(&mut entry, &mut out_file, done, total, ctx)?;
            count += 1;
        }
    }

    Ok(count)
}

fn safe_archive_path(destination: &Path, name: &str) -> Result<PathBuf> {
    let normalized = name.replace('\\', "/");
    let relative = Path::new(&normalized);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "unsafe archive path: {name}"
        )));
    }
    Ok(destination.join(relative))
}

fn extract_tar(path: &Path, dest: &Path, format: Format, ctx: &ProgressCtx) -> Result<usize> {
    let total = std::fs::metadata(path)?.len().max(1);
    let file = File::open(path)?;
    let reader: Box<dyn Read> = match format {
        Format::TarGz => Box::new(flate2::read::GzDecoder::new(file)),
        Format::TarXz => Box::new(xz2::read::XzDecoder::new(file)),
        Format::TarZst => Box::new(zstd::stream::Decoder::new(file)?),
        Format::TarBz2 => Box::new(bzip2::read::BzDecoder::new(file)),
        Format::Tar => Box::new(file),
        _ => unreachable!(),
    };
    let mut archive = tar::Archive::new(reader);
    let mut count = 0u64;
    for entry in archive.entries()? {
        let mut entry = entry?;
        entry.unpack_in(dest)?;
        count += 1;
        // tar entries stream from a compressed reader, so per-byte progress
        // against compressed file size isn't meaningful — tick coarsely
        // per-entry instead (still real cancel responsiveness, since
        // ctx.tick still checks the cancel flag every entry).
        ctx.tick(count.min(total), total)?;
    }
    Ok(count as usize)
}

fn extract_7z(path: &Path, dest: &Path, password: Option<&str>) -> Result<usize> {
    let pw = sevenz_rust::Password::from(password.unwrap_or(""));
    sevenz_rust::decompress_file_with_password(path, dest, pw.clone())
        .map_err(|e| ArchiveError::SevenZ(e.to_string()))?;

    let archive = sevenz_rust::Archive::open_with_password(path, &pw)
        .map_err(|e| ArchiveError::SevenZ(e.to_string()))?;
    Ok(archive.files.len())
}

/// Verify every entry's CRC-32 matches its stored value (zip "Test archive").
pub fn test_archive(path: &str) -> Result<bool> {
    let path = Path::new(path);
    let format = Format::from_path(path)
        .ok_or_else(|| ArchiveError::UnsupportedFormat(path.display().to_string()))?;

    if !matches!(format, Format::Init | Format::Zip) {
        // Non-zip formats: fall back to a full-read sanity check.
        // .to_string_lossy() instead of .to_str().unwrap() — an unwrap here
        // would panic (and on Windows a main-thread-adjacent panic like this
        // can take the whole app down) on the rare path that isn't valid
        // UTF-8. Lossy conversion just means a lossy display string for the
        // one code path (fallback full-read sanity check) that needs it.
        list_archive(&path.to_string_lossy(), None)?;
        return Ok(true);
    }

    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
    for i in 0..zip.len() {
        let mut entry = match zip.by_index(i) {
            Ok(e) => e,
            Err(zip::result::ZipError::UnsupportedArchive(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        if entry.is_dir() {
            continue;
        }
        let mut buf = [0u8; 8192];
        let mut hasher = crc32fast::Hasher::new();
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        if hasher.finalize() != entry.crc32() {
            return Err(ArchiveError::IntegrityFailed(entry.name().to_string()));
        }
    }
    Ok(true)
}

/// Remove entries from an existing zip/.init archive by rebuilding it
/// without them. Deliberately uses the same decompress-then-recompress
/// path as everywhere else in this file (open each surviving entry, read
/// its plain bytes, write it back through the normal `start_file` API)
/// rather than a raw-copy of the original compressed bytes — raw-copying
/// is faster but needs a lower-level zip-crate API this project hasn't
/// verified compiles correctly without a working `cargo build` on hand,
/// and getting that wrong risks a corrupt archive. This is slower for
/// large files but uses only patterns already proven elsewhere in this
/// codebase.
pub fn delete_entries(path: &str, names: &[String], ctx: &ProgressCtx) -> Result<usize> {
    let format = Format::from_path(Path::new(path))
        .ok_or_else(|| ArchiveError::UnsupportedFormat(path.to_string()))?;
    if !matches!(format, Format::Init | Format::Zip) {
        return Err(ArchiveError::UnsupportedFormat(
            "deleting entries is only supported for .init/.zip archives".to_string(),
        ));
    }

    let names_to_remove: std::collections::HashSet<&str> =
        names.iter().map(|n| n.as_str()).collect();

    let temp_path = PathBuf::from(format!("{path}.syncinit-part-{}", std::process::id()));
    let result = delete_entries_into(path, &names_to_remove, ctx, &temp_path);
    match result {
        Ok(count) => {
            std::fs::remove_file(path)?;
            std::fs::rename(&temp_path, path)?;
            Ok(count)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(e)
        }
    }
}

fn delete_entries_into(
    path: &str,
    names_to_remove: &std::collections::HashSet<&str>,
    ctx: &ProgressCtx,
    temp_path: &Path,
) -> Result<usize> {
    let src_file = File::open(path)?;
    let mut reader = zip::ZipArchive::new(BufReader::new(src_file))?;

    let out_file = BufWriter::new(File::create(temp_path)?);
    let mut writer = zip::ZipWriter::new(out_file);
    if !reader.comment().is_empty() {
        writer.set_comment(String::from_utf8_lossy(reader.comment()).into_owned());
    }

    let total = reader.len() as u64;
    let mut removed = 0usize;

    for i in 0..reader.len() {
        ctx.tick(i as u64, total.max(1))?;
        let mut entry = reader.by_index(i)?;
        if names_to_remove.contains(entry.name()) {
            removed += 1;
            continue;
        }

        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(entry.compression())
            .unix_permissions(entry.unix_mode().unwrap_or(0o644));

        if entry.is_dir() {
            writer.add_directory(entry.name().to_string(), options)?;
        } else {
            writer.start_file(entry.name().to_string(), options)?;
            std::io::copy(&mut entry, &mut writer)?;
        }
    }

    writer
        .finish()?
        .into_inner()
        .map_err(|e| e.into_error())?
        .flush()?;
    Ok(removed)
}

/// Delete the original inputs only after an archive has been created and the
/// caller has explicitly confirmed the destructive action in the UI.
pub fn delete_sources(sources: &[String], destination: &str) -> Result<usize> {
    let destination = std::fs::canonicalize(destination)?;
    let mut deleted = 0;
    for source in sources {
        let path = std::fs::canonicalize(source)?;
        if path.is_dir() && destination.starts_with(&path) {
            return Err(ArchiveError::UnsafeDelete);
        }
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        deleted += 1;
    }
    Ok(deleted)
}
