use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
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
}

pub type Result<T> = std::result::Result<T, ArchiveError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Arc,
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
        } else if name.ends_with(".arc") {
            Some(Format::Arc)
        } else if name.ends_with(".zip") {
            Some(Format::Zip)
        } else {
            None
        }
    }

    /// Formats Zarc can write natively. Other formats are read/extract-only.
    pub fn is_writable(&self) -> bool {
        matches!(
            self,
            Format::Arc | Format::Zip | Format::Tar | Format::TarGz | Format::TarZst
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
        Format::Arc => list_zip(path, Format::Arc, password),
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
/// encrypted .arc/.zip always requires the password.
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

/// Create a new archive from a set of source files/directories.
pub fn create_archive(opts: &CreateOptions) -> Result<()> {
    if !opts.format.is_writable() {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "{:?} writing not implemented yet",
            opts.format
        )));
    }

    match opts.format {
        Format::Arc | Format::Zip => create_zip(opts),
        Format::Tar => create_tar(opts, None),
        Format::TarGz => create_tar(opts, Some(CompressorKind::Gzip)),
        Format::TarZst => create_tar(opts, Some(CompressorKind::Zstd)),
        _ => unreachable!(),
    }
}

enum CompressorKind {
    Gzip,
    Zstd,
}

fn create_zip(opts: &CreateOptions) -> Result<()> {
    let destination = Path::new(&opts.destination);
    let temp_path = destination.with_extension(format!(
        "{}.zarc-part-{}",
        destination
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("arc"),
        std::process::id()
    ));
    let file = BufWriter::new(File::create(&temp_path)?);
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
            for entry in walkdir::WalkDir::new(&src_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
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
                    writer.start_file(zip_path, zip_options(opts, should_store(path, opts)))?;
                    let mut f = BufReader::with_capacity(128 * 1024, File::open(path)?);
                    std::io::copy(&mut f, &mut writer)?;
                }
            }
        } else {
            writer.start_file(base_name, zip_options(opts, should_store(&src_path, opts)))?;
            let mut f = BufReader::with_capacity(128 * 1024, File::open(&src_path)?);
            std::io::copy(&mut f, &mut writer)?;
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
    std::fs::rename(&temp_path, destination)?;
    Ok(())
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
            "7z" | "arc"
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

fn create_tar(opts: &CreateOptions, compressor: Option<CompressorKind>) -> Result<()> {
    let file = File::create(&opts.destination)?;
    let writer: Box<dyn Write> = match compressor {
        Some(CompressorKind::Gzip) => Box::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::new(opts.level.min(9) as u32),
        )),
        Some(CompressorKind::Zstd) => {
            Box::new(zstd::stream::Encoder::new(file, opts.level.min(19) as i32)?.auto_finish())
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
            tar_builder.append_dir_all(&base_name, &src_path)?;
        } else {
            let mut f = File::open(&src_path)?;
            tar_builder.append_file(&base_name, &mut f)?;
        }
    }

    tar_builder.finish()?;
    Ok(())
}

/// Extract an archive to a destination directory. Supports zip, tar variants,
/// and 7z (read-only). Returns the number of entries extracted.
pub fn extract_archive(path: &str, destination: &str, password: Option<&str>) -> Result<usize> {
    let path = Path::new(path);
    let dest = Path::new(destination);
    std::fs::create_dir_all(dest)?;

    let format = Format::from_path(path)
        .ok_or_else(|| ArchiveError::UnsupportedFormat(path.display().to_string()))?;

    match format {
        Format::Arc | Format::Zip => extract_zip(path, dest, password),
        Format::Tar | Format::TarGz | Format::TarXz | Format::TarZst | Format::TarBz2 => {
            extract_tar(path, dest, format)
        }
        Format::SevenZ => extract_7z(path, dest, password),
    }
}

fn extract_zip(path: &Path, dest: &Path, password: Option<&str>) -> Result<usize> {
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file))?;
    let mut count = 0;

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
            std::io::copy(&mut entry, &mut out_file)?;
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

fn extract_tar(path: &Path, dest: &Path, format: Format) -> Result<usize> {
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
    archive.unpack(dest)?;
    let count = tar::Archive::new(File::open(path).map(|f| f)?)
        .entries()?
        .count();
    Ok(count)
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

    if !matches!(format, Format::Arc | Format::Zip) {
        // Non-zip formats: fall back to a full-read sanity check.
        list_archive(path.to_str().unwrap(), None)?;
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
