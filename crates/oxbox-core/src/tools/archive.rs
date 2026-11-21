//! Tool: **Archive & Unarchive** (`archive`).

use crate::{
    CoreError, CoreResult,
    fs_safe::reject_symlink_components,
    jobs::{CancelToken, Progress},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
mod seven_z_guard;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveFormat {
    Zip,
    TarGz,
}

fn archive_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("archive: {error}"))
}

/// Treat both path separators as separators on every OS. Windows device names,
/// alternate data streams and trailing-dot aliases are also rejected on Unix.
pub fn safe_entry_path(name: &str) -> CoreResult<PathBuf> {
    let name = name.replace('\\', "/");
    if name.is_empty() || name.starts_with('/') || name.contains(':') || name.contains('\0') {
        return Err(archive_error("entry has an unsafe path"));
    }
    let mut result = PathBuf::new();
    for component in name.split('/').filter(|s| !s.is_empty()) {
        crate::fs_safe::validate_portable_filename(component)?;
        result.push(component);
    }
    if result.as_os_str().is_empty() {
        return Err(archive_error("entry has an empty path"));
    }
    Ok(result)
}

fn source_entries(
    sources: &[PathBuf],
    output: &Path,
    cancel: &CancelToken,
) -> CoreResult<Vec<(PathBuf, PathBuf, u64, bool)>> {
    if sources.is_empty() {
        return Err(archive_error("choose files or folders first"));
    }
    let output_parent = output
        .parent()
        .ok_or_else(|| archive_error("choose an output location"))?;
    reject_symlink_components(output_parent)?;
    let output = fs::canonicalize(output_parent)?.join(
        output
            .file_name()
            .ok_or_else(|| archive_error("output needs a file name"))?,
    );
    let mut entries = Vec::new();
    let mut names = HashSet::new();
    for source in sources {
        cancel.check()?;
        reject_symlink_components(source)?;
        let source = fs::canonicalize(source)?;
        if source.is_dir() && output.starts_with(&source) {
            return Err(archive_error("save the archive outside its source folders"));
        }
        let parent = source
            .parent()
            .ok_or_else(|| archive_error("disk roots cannot be archived"))?;
        for entry in WalkDir::new(&source).follow_links(false) {
            cancel.check()?;
            let entry = entry.map_err(archive_error)?;
            if entries.len() >= 100_000 {
                return Err(archive_error(
                    "an archive can contain at most 100,000 entries",
                ));
            }
            if entry.file_type().is_symlink() {
                return Err(archive_error("symbolic links are not archived"));
            }
            if !entry.file_type().is_dir() && !entry.file_type().is_file() {
                return Err(archive_error(
                    "only regular files and folders can be archived",
                ));
            }
            let relative = entry.path().strip_prefix(parent).map_err(archive_error)?;
            let name = safe_entry_path(&relative.to_string_lossy())?;
            if !names.insert(name.to_string_lossy().to_lowercase()) {
                return Err(archive_error(
                    "two source entries have the same archive name",
                ));
            }
            let size = if entry.file_type().is_file() {
                entry.metadata().map_err(archive_error)?.len()
            } else {
                0
            };
            entries.push((
                entry.path().to_path_buf(),
                name,
                size,
                entry.file_type().is_dir(),
            ));
        }
    }
    Ok(entries)
}

struct JobReader<'a, R> {
    reader: R,
    cancel: &'a CancelToken,
    progress: &'a mut dyn FnMut(Progress),
    done: &'a mut u64,
    total: u64,
}
impl<R: Read> Read for JobReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        // io::copy retries Interrupted forever; cancellation must end the copy.
        self.cancel
            .check()
            .map_err(|_| std::io::Error::other("Cancelled"))?;
        let n = self.reader.read(buffer)?;
        *self.done = self.done.saturating_add(n as u64);
        (self.progress)(Progress {
            done: *self.done,
            total: self.total,
            message: "Creating archive".into(),
        });
        Ok(n)
    }
}

pub fn create_archive(
    sources: &[PathBuf],
    output: &Path,
    format: ArchiveFormat,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<PathBuf> {
    cancel.check()?;
    reject_symlink_components(output)?;
    crate::fs_safe::validate_output_filename(
        output
            .file_name()
            .ok_or_else(|| archive_error("output needs a file name"))?,
    )?;
    let entries = source_entries(sources, output, cancel)?;
    let total = entries.iter().fold(0_u64, |sum, e| sum.saturating_add(e.2));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut done = 0;
    let result: CoreResult<()> = (|| {
        match format {
            ArchiveFormat::Zip => {
                let mut writer = zip::ZipWriter::new(file);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(0o600);
                for (path, name, size, is_dir) in &entries {
                    cancel.check()?;
                    let name = name.to_string_lossy().replace('\\', "/");
                    if *is_dir {
                        writer
                            .add_directory(format!("{name}/"), options)
                            .map_err(archive_error)?;
                    } else {
                        reject_symlink_components(path)?;
                        writer
                            .start_file(name, options.large_file(*size >= u32::MAX as u64))
                            .map_err(archive_error)?;
                        let mut reader = JobReader {
                            reader: BufReader::new(File::open(path)?),
                            cancel,
                            progress,
                            done: &mut done,
                            total,
                        };
                        std::io::copy(&mut reader, &mut writer)?;
                    }
                }
                writer.finish().map_err(archive_error)?.sync_all()?;
            }
            ArchiveFormat::TarGz => {
                let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
                let mut writer = tar::Builder::new(encoder);
                for (path, name, size, is_dir) in &entries {
                    cancel.check()?;
                    let mut header = tar::Header::new_gnu();
                    header.set_mode(if *is_dir { 0o700 } else { 0o600 });
                    header.set_entry_type(if *is_dir {
                        tar::EntryType::Directory
                    } else {
                        tar::EntryType::Regular
                    });
                    header.set_size(*size);
                    header.set_cksum();
                    if *is_dir {
                        writer.append_data(&mut header, name, std::io::empty())?;
                    } else {
                        reject_symlink_components(path)?;
                        let reader = JobReader {
                            reader: BufReader::new(File::open(path)?),
                            cancel,
                            progress,
                            done: &mut done,
                            total,
                        };
                        writer.append_data(&mut header, name, reader)?;
                    }
                }
                writer.into_inner()?.finish()?.sync_all()?;
            }
        }
        cancel.check()?;
        Ok(())
    })();
    if let Err(error) = result {
        // Only our freshly-created output is removed, never an existing user file.
        let _ = fs::remove_file(output);
        cancel.check()?;
        return Err(error);
    }
    progress(Progress {
        done: total,
        total,
        message: "Archive created".into(),
    });
    Ok(output.to_path_buf())
}

struct Extractor<'a> {
    dest: &'a Path,
    cancel: &'a CancelToken,
    progress: &'a mut dyn FnMut(Progress),
    limit: u64,
    done: u64,
    entries: usize,
    names: HashSet<String>,
}
impl Extractor<'_> {
    fn entry(
        &mut self,
        name: &str,
        size: u64,
        is_dir: bool,
        reader: &mut dyn Read,
    ) -> CoreResult<()> {
        self.cancel.check()?;
        self.entries += 1;
        if self.entries > 100_000 {
            return Err(archive_error("archive has more than 100,000 entries"));
        }
        let relative = safe_entry_path(name)?;
        let key = relative.to_string_lossy().replace('\\', "/").to_lowercase();
        if !self.names.insert(key) {
            return Err(archive_error("archive contains duplicate entry paths"));
        }
        if size > self.limit.saturating_sub(self.done) {
            return Err(archive_error(
                "extracted size exceeds the confirmed limit or 20× compressed size",
            ));
        }
        let path = self.dest.join(relative);
        reject_symlink_components(&path)?;
        if is_dir {
            if path.exists() && !path.is_dir() {
                return Err(archive_error("directory conflicts with a file"));
            }
            fs::create_dir_all(&path)?;
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        reject_symlink_components(&path)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        let mut bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            self.cancel.check()?;
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            bytes = bytes.saturating_add(n as u64);
            if bytes > size || n as u64 > self.limit.saturating_sub(self.done) {
                return Err(archive_error(
                    "entry exceeds its declared size or extraction budget",
                ));
            }
            output.write_all(&buffer[..n])?;
            self.done += n as u64;
            (self.progress)(Progress {
                done: self.done,
                total: self.limit,
                message: format!("Extracting {name}"),
            });
        }
        if bytes != size {
            return Err(archive_error("entry data is truncated"));
        }
        Ok(())
    }
}

// tar::Archive preprocesses GNU/PAX records before yielding an entry and may
// otherwise grow a metadata buffer without reaching our extraction budgets.
// This guard validates raw framing first while leaving format interpretation to
// the tar crate. Its metadata buffer is bounded independently of file contents.
struct TarGuard<'a, R> {
    reader: R,
    cancel: &'a CancelToken,
    header: [u8; 512],
    header_offset: usize,
    payload: u64,
    padding: u64,
    metadata: Option<Vec<u8>>,
    pax_metadata: bool,
    pax_size: Option<u64>,
    metadata_total: u64,
    headers: u64,
    ended: bool,
}
impl<'a, R: Read> TarGuard<'a, R> {
    fn new(reader: R, cancel: &'a CancelToken) -> Self {
        Self {
            reader,
            cancel,
            header: [0; 512],
            header_offset: 512,
            payload: 0,
            padding: 0,
            metadata: None,
            pax_metadata: false,
            pax_size: None,
            metadata_total: 0,
            headers: 0,
            ended: false,
        }
    }
    fn invalid(message: &str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, message)
    }
    fn finish_metadata(&mut self) -> std::io::Result<()> {
        if let Some(bytes) = self.metadata.take()
            && self.pax_metadata
        {
            for extension in tar::PaxExtensions::new(&bytes) {
                let extension = extension?;
                if extension.key_bytes().starts_with(b"GNU.sparse.") {
                    return Err(Self::invalid("TAR sparse files are not extracted"));
                }
                if extension.key_bytes() == b"size" && self.pax_size.is_none() {
                    self.pax_size = Some(
                        extension
                            .value()
                            .map_err(|_| Self::invalid("invalid TAR PAX size encoding"))?
                            .parse()
                            .map_err(|_| Self::invalid("invalid TAR PAX size"))?,
                    );
                }
            }
        }
        self.pax_metadata = false;
        Ok(())
    }
    fn next_header(&mut self) -> std::io::Result<()> {
        self.reader.read_exact(&mut self.header)?;
        self.header_offset = 0;
        if self.header.iter().all(|byte| *byte == 0) {
            self.ended = true;
            return Ok(());
        }
        self.headers += 1;
        if self.headers > 200_000 {
            return Err(Self::invalid("TAR has too many physical headers"));
        }
        let header = tar::Header::from_byte_slice(&self.header);
        let kind = header.entry_type();
        let metadata = kind.is_gnu_longname() || kind.is_pax_local_extensions();
        if !metadata && !kind.is_file() && !kind.is_dir() {
            return Err(Self::invalid(
                "TAR links, sparse files and special records are not extracted",
            ));
        }
        let size = if metadata {
            header.entry_size()?
        } else {
            self.pax_size.take().unwrap_or(header.entry_size()?)
        };
        if metadata {
            self.metadata_total = self.metadata_total.saturating_add(size);
            if size > 64 * 1024 || self.metadata_total > 4 * 1024 * 1024 {
                return Err(Self::invalid(
                    "TAR extension metadata exceeds the 64 KiB per-record or 4 MiB archive limit",
                ));
            }
            self.metadata = Some(Vec::with_capacity(size as usize));
            self.pax_metadata = kind.is_pax_local_extensions();
        } else if kind.is_dir() && size != 0 {
            return Err(Self::invalid("TAR directories cannot contain a payload"));
        }
        self.payload = size;
        self.padding = (512 - size % 512) % 512;
        if size == 0 {
            self.finish_metadata()?;
        }
        Ok(())
    }
}
impl<R: Read> Read for TarGuard<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        self.cancel
            .check()
            .map_err(|_| std::io::Error::other("Cancelled"))?;
        if self.header_offset < 512 {
            let count = out.len().min(512 - self.header_offset);
            out[..count]
                .copy_from_slice(&self.header[self.header_offset..self.header_offset + count]);
            self.header_offset += count;
            return Ok(count);
        }
        if self.payload > 0 {
            let limit = out
                .len()
                .min(64 * 1024)
                .min(self.payload.min(usize::MAX as u64) as usize);
            let count = self.reader.read(&mut out[..limit])?;
            if count == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }
            self.payload -= count as u64;
            if let Some(bytes) = &mut self.metadata {
                bytes.extend_from_slice(&out[..count]);
            }
            if self.payload == 0 {
                self.finish_metadata()?;
            }
            return Ok(count);
        }
        if self.padding > 0 {
            let limit = out.len().min(self.padding as usize);
            let count = self.reader.read(&mut out[..limit])?;
            if count == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }
            self.padding -= count as u64;
            return Ok(count);
        }
        if self.ended {
            return Ok(0);
        }
        self.next_header()?;
        self.read(out)
    }
}

fn extract_tar<R: Read>(reader: R, extractor: &mut Extractor<'_>) -> CoreResult<()> {
    let result = (|| {
        let mut archive = tar::Archive::new(TarGuard::new(reader, extractor.cancel));
        for entry in archive.entries()? {
            extractor.cancel.check()?;
            let mut entry = entry?;
            let kind = entry.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err(archive_error("links and special files are not extracted"));
            }
            let name = entry
                .path()?
                .to_str()
                .ok_or_else(|| archive_error("entry name is not valid Unicode"))?
                .to_owned();
            extractor.entry(&name, entry.size(), kind.is_dir(), &mut entry)?;
        }
        Ok(())
    })();
    extractor.cancel.check()?;
    result
}

/// Bound directory metadata before ZipArchive allocates its entry table.
fn preflight_zip(file: &mut File) -> CoreResult<()> {
    let length = file.metadata()?.len();
    let tail_len = length.min(65_557) as usize;
    let mut tail = vec![0_u8; tail_len];
    file.seek(SeekFrom::End(-(tail_len as i64)))?;
    file.read_exact(&mut tail)?;
    let offset = (0..tail_len.saturating_sub(21))
        .rev()
        .find(|&i| {
            tail[i..i + 4] == *b"PK\x05\x06"
                && i + 22 + u16::from_le_bytes([tail[i + 20], tail[i + 21]]) as usize == tail_len
        })
        .ok_or_else(|| archive_error("ZIP end-of-directory record was not found"))?;
    let end = &tail[offset..offset + 22];
    if end[4..8] != [0, 0, 0, 0] {
        return Err(archive_error("multi-volume ZIP archives are not supported"));
    }
    let mut entries = u16::from_le_bytes([end[10], end[11]]) as u64;
    let entries_on_disk = u16::from_le_bytes([end[8], end[9]]) as u64;
    let mut directory_size = u32::from_le_bytes([end[12], end[13], end[14], end[15]]) as u64;
    let mut directory_offset = u32::from_le_bytes([end[16], end[17], end[18], end[19]]) as u64;
    let absolute = length - tail_len as u64 + offset as u64;
    let mut directory_end = absolute;
    if entries == u16::MAX as u64
        || entries_on_disk == u16::MAX as u64
        || directory_size == u32::MAX as u64
        || directory_offset == u32::MAX as u64
    {
        if absolute < 20 {
            return Err(archive_error("ZIP64 directory locator is missing"));
        }
        file.seek(SeekFrom::Start(absolute - 20))?;
        let mut locator = [0_u8; 20];
        file.read_exact(&mut locator)?;
        if locator[..4] != *b"PK\x06\x07"
            || locator[4..8] != [0, 0, 0, 0]
            || locator[16..20] != [1, 0, 0, 0]
        {
            return Err(archive_error("invalid or multi-volume ZIP64 locator"));
        }
        let mut number = [0_u8; 8];
        number.copy_from_slice(&locator[8..16]);
        let record_offset = u64::from_le_bytes(number);
        if record_offset > length.saturating_sub(56) {
            return Err(archive_error("ZIP64 directory points beyond the archive"));
        }
        file.seek(SeekFrom::Start(record_offset))?;
        let mut record = [0_u8; 56];
        file.read_exact(&mut record)?;
        if record[..4] != *b"PK\x06\x06" || record[16..24] != [0; 8] {
            return Err(archive_error("invalid or multi-volume ZIP64 directory"));
        }
        number.copy_from_slice(&record[4..12]);
        let record_size = u64::from_le_bytes(number);
        if !(44..=1024 * 1024).contains(&record_size)
            || record_offset
                .checked_add(record_size)
                .and_then(|n| n.checked_add(12))
                != Some(absolute - 20)
        {
            return Err(archive_error(
                "ZIP64 extensible metadata exceeds 1 MiB or has inconsistent framing",
            ));
        }
        number.copy_from_slice(&record[32..40]);
        entries = u64::from_le_bytes(number);
        number.copy_from_slice(&record[24..32]);
        if u64::from_le_bytes(number) != entries {
            return Err(archive_error("ZIP64 entry counts are inconsistent"));
        }
        number.copy_from_slice(&record[40..48]);
        directory_size = u64::from_le_bytes(number);
        number.copy_from_slice(&record[48..56]);
        directory_offset = u64::from_le_bytes(number);
        directory_end = record_offset;
    } else if entries_on_disk != entries {
        return Err(archive_error("ZIP entry counts are inconsistent"));
    }
    // Require one contiguous, unprefixed central directory. Patched zip refuses
    // to fall back past an abutting footer, so its parser cannot choose an older
    // footer with metadata that was never checked against the limits above.
    if directory_offset.checked_add(directory_size) != Some(directory_end)
        || entries.saturating_mul(46) > directory_size
    {
        return Err(archive_error(
            "ZIP central directory has an unsupported or inconsistent layout",
        ));
    }
    if entries > 100_000 || directory_size > 64 * 1024 * 1024 || directory_size > length {
        return Err(archive_error(
            "ZIP directory exceeds the metadata limits (100,000 entries / 64 MiB)",
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

pub fn extract_archive(
    input: &Path,
    output: &Path,
    max_bytes: u64,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<PathBuf> {
    cancel.check()?;
    if max_bytes == 0 {
        return Err(archive_error("extraction limit must be positive"));
    }
    reject_symlink_components(input)?;
    reject_symlink_components(output)?;
    if !fs::metadata(input)?.is_file() {
        return Err(archive_error("choose a regular archive file"));
    }
    let mut file = File::open(input)?;
    let compressed = file.metadata()?.len();
    let mut magic = [0_u8; 6];
    let n = file.read(&mut magic)?;
    file.seek(SeekFrom::Start(0))?;
    let is_zip = n >= 4 && magic[..2] == *b"PK";
    let is_gz = n >= 2 && magic[..2] == [0x1f, 0x8b];
    let is_7z = n == 6 && magic == [b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c];
    if is_zip {
        preflight_zip(&mut file)?;
    }
    let seven_z_metadata = if is_7z {
        Some(seven_z_guard::preflight(&mut file, cancel)?)
    } else {
        None
    };
    // A new destination is required: extraction never merges into a populated tree.
    // Any cancelled/failed extraction leaves only this newly-created partial folder.
    fs::create_dir(output)?;
    let output = fs::canonicalize(output)?;
    let limit = max_bytes.min(compressed.saturating_mul(20));
    let mut extractor = Extractor {
        dest: &output,
        cancel,
        progress,
        limit,
        done: 0,
        entries: 0,
        names: HashSet::new(),
    };
    if is_zip {
        let mut archive = zip::ZipArchive::new(file).map_err(archive_error)?;
        if archive.len() > 100_000 {
            return Err(archive_error("archive has more than 100,000 entries"));
        }
        for index in 0..archive.len() {
            cancel.check()?;
            let mut entry = archive
                .by_index_with_options(
                    index,
                    zip::ZipReadOptions::new().enforce_header_consistency(true),
                )
                .map_err(archive_error)?;
            if entry.unix_mode().is_some_and(|mode| {
                let kind = mode & 0o170000;
                kind != 0 && kind != 0o100000 && kind != 0o040000
            }) {
                return Err(archive_error("links and special files are not extracted"));
            }
            let name = entry.name().map_err(archive_error)?.into_owned();
            extractor.entry(&name, entry.size(), entry.is_dir(), &mut entry)?;
        }
    } else if is_gz {
        extract_tar(flate2::read::GzDecoder::new(file), &mut extractor)?;
    } else if let Some(metadata) = seven_z_metadata {
        let mut archive = metadata.reader(file, cancel)?;
        let mut callback_error = None;
        archive
            .for_each_entries(|entry, reader| {
                if callback_error.is_some() {
                    return Err(sevenz_rust2::Error::Other("extraction stopped".into()));
                }
                let result = if entry.is_anti_item
                    || (entry.has_windows_attributes && entry.windows_attributes & 0x400 != 0)
                {
                    Err(archive_error(
                        "links and deletion entries are not extracted",
                    ))
                } else {
                    extractor.entry(entry.name(), entry.size(), entry.is_directory(), reader)
                };
                if let Err(error) = result {
                    callback_error = Some(error);
                    return Err(sevenz_rust2::Error::Other("extraction stopped".into()));
                }
                Ok(true)
            })
            .map_err(|e| callback_error.take().unwrap_or_else(|| archive_error(e)))?;
    } else {
        extract_tar(file, &mut extractor)?;
    }
    cancel.check()?;
    let done = extractor.done;
    (extractor.progress)(Progress {
        done,
        total: done,
        message: "Extraction complete".into(),
    });
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_tar_entry(
        bytes: &mut Vec<u8>,
        kind: tar::EntryType,
        name: &str,
        declared_size: u64,
        payload: &[u8],
    ) {
        let mut header = tar::Header::new_gnu();
        header.set_path(name).unwrap();
        header.set_entry_type(kind);
        header.set_size(declared_size);
        header.set_mode(0o600);
        header.set_cksum();
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(payload);
        bytes.resize(bytes.len() + (512 - payload.len() % 512) % 512, 0);
    }
    fn pax_record(key: &str, value: &str) -> String {
        let body = format!(" {key}={value}\n");
        let mut length = body.len() + 1;
        loop {
            let next = body.len() + length.to_string().len();
            if next == length {
                return format!("{length}{body}");
            }
            length = next;
        }
    }
    fn fixture_extractor<'a>(
        dest: &'a Path,
        cancel: &'a CancelToken,
        progress: &'a mut dyn FnMut(Progress),
    ) -> Extractor<'a> {
        Extractor {
            dest,
            cancel,
            progress,
            limit: 1_048_576,
            done: 0,
            entries: 0,
            names: HashSet::new(),
        }
    }

    #[test]
    fn tar_metadata_is_bounded_before_the_parser_reads_its_payload() {
        use std::{cell::Cell, io::Cursor, rc::Rc};
        struct HeaderOnly {
            header: Cursor<Vec<u8>>,
            payload_reads: Rc<Cell<u32>>,
        }
        impl Read for HeaderOnly {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.header.position() >= 512 {
                    self.payload_reads.set(self.payload_reads.get() + 1);
                    return Err(std::io::Error::other("unexpected payload read"));
                }
                self.header.read(out)
            }
        }
        let temp = tempfile::tempdir().unwrap();
        for kind in [tar::EntryType::GNULongName, tar::EntryType::XHeader] {
            let mut header = tar::Header::new_gnu();
            header.set_path("metadata").unwrap();
            header.set_entry_type(kind);
            header.set_size(1024 * 1024);
            header.set_cksum();
            let payload_reads = Rc::new(Cell::new(0));
            let reader = HeaderOnly {
                header: Cursor::new(header.as_bytes().to_vec()),
                payload_reads: payload_reads.clone(),
            };
            let cancel = CancelToken::default();
            let mut progress = |_| {};
            let mut extractor = fixture_extractor(temp.path(), &cancel, &mut progress);
            let error = extract_tar(reader, &mut extractor).unwrap_err();
            assert!(
                error.to_string().contains("TAR extension metadata"),
                "{error}"
            );
            assert_eq!(
                payload_reads.get(),
                0,
                "oversized metadata must be refused before a payload read/allocation"
            );
        }
    }

    #[test]
    fn tar_pax_path_and_size_override_remain_supported() {
        let mut bytes = Vec::new();
        let pax = format!(
            "{}{}",
            pax_record("path", "nested/pax.txt"),
            pax_record("size", "5")
        );
        raw_tar_entry(
            &mut bytes,
            tar::EntryType::XHeader,
            "pax",
            pax.len() as u64,
            pax.as_bytes(),
        );
        raw_tar_entry(&mut bytes, tar::EntryType::Regular, "ignored", 0, b"hello");
        bytes.extend_from_slice(&[0; 1024]);
        let temp = tempfile::tempdir().unwrap();
        let cancel = CancelToken::default();
        let mut progress = |_| {};
        let mut extractor = fixture_extractor(temp.path(), &cancel, &mut progress);
        extract_tar(std::io::Cursor::new(bytes), &mut extractor).unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("nested/pax.txt")).unwrap(),
            b"hello"
        );
    }

    #[test]
    fn tar_metadata_reads_observe_cancellation() {
        struct CancelReader {
            reader: std::io::Cursor<Vec<u8>>,
            cancel: CancelToken,
            read: std::rc::Rc<std::cell::Cell<u64>>,
        }
        impl Read for CancelReader {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let limit = out.len().min(4096);
                let n = self.reader.read(&mut out[..limit])?;
                self.read.set(self.read.get() + n as u64);
                if self.read.get() >= 1024 {
                    self.cancel.cancel();
                }
                Ok(n)
            }
        }
        let mut bytes = Vec::new();
        let mut name = vec![b'a'; 32 * 1024];
        name.push(0);
        raw_tar_entry(
            &mut bytes,
            tar::EntryType::GNULongName,
            "longname",
            name.len() as u64,
            &name,
        );
        raw_tar_entry(&mut bytes, tar::EntryType::Regular, "ignored", 0, &[]);
        bytes.extend_from_slice(&[0; 1024]);
        let temp = tempfile::tempdir().unwrap();
        let cancel = CancelToken::default();
        let read = std::rc::Rc::new(std::cell::Cell::new(0));
        let reader = CancelReader {
            reader: std::io::Cursor::new(bytes),
            cancel: cancel.clone(),
            read: read.clone(),
        };
        let mut progress = |_| {};
        let mut extractor = fixture_extractor(temp.path(), &cancel, &mut progress);
        assert_eq!(
            extract_tar(reader, &mut extractor).unwrap_err().code(),
            "cancelled"
        );
        assert!(
            read.get() < 16 * 1024,
            "metadata preprocessing must not continue reading after cancellation"
        );
    }

    #[test]
    fn tar_gnu_long_filename_round_trip_is_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let name = format!("{}.txt", "original-name-".repeat(10));
        let source = temp.path().join(&name);
        std::fs::write(&source, b"contents").unwrap();
        let archive = temp.path().join("long.tar.gz");
        create_archive(
            &[source],
            &archive,
            ArchiveFormat::TarGz,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        let dest = temp.path().join("output");
        extract_archive(
            &archive,
            &dest,
            1_048_576,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(std::fs::read(dest.join(name)).unwrap(), b"contents");
    }

    #[test]
    fn rejects_traversal_absolute_windows_and_device_names() {
        for name in [
            "../escape",
            "a/../../escape",
            "/absolute",
            "C:\\escape",
            "a\\..\\escape",
            "file:stream",
            "CON",
            "dir/NUL.txt",
        ] {
            assert!(safe_entry_path(name).is_err(), "{name}");
        }
        assert_eq!(
            safe_entry_path("folder/file.txt").unwrap(),
            PathBuf::from("folder/file.txt")
        );
    }

    #[test]
    fn zip_and_tar_gzip_round_trip_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("hello.txt");
        std::fs::write(&input, b"original contents").unwrap();
        for (index, format) in [ArchiveFormat::Zip, ArchiveFormat::TarGz]
            .into_iter()
            .enumerate()
        {
            let archive = temp.path().join(format!("archive-{index}"));
            create_archive(
                std::slice::from_ref(&input),
                &archive,
                format,
                &CancelToken::default(),
                &mut |_| {},
            )
            .unwrap();
            let output = temp.path().join(format!("output-{index}"));
            extract_archive(
                &archive,
                &output,
                1_048_576,
                &CancelToken::default(),
                &mut |_| {},
            )
            .unwrap();
            assert_eq!(
                std::fs::read(output.join("hello.txt")).unwrap(),
                b"original contents"
            );
            assert!(
                extract_archive(
                    &archive,
                    &output,
                    1_048_576,
                    &CancelToken::default(),
                    &mut |_| {}
                )
                .is_err()
            );
            assert!(
                create_archive(
                    std::slice::from_ref(&input),
                    &archive,
                    format,
                    &CancelToken::default(),
                    &mut |_| {}
                )
                .is_err()
            );
        }
    }

    #[test]
    fn extraction_enforces_size_budget() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("big");
        std::fs::write(&file, vec![0; 256 * 1024]).unwrap();
        let archive = temp.path().join("bomb.zip");
        create_archive(
            &[file],
            &archive,
            ArchiveFormat::Zip,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(
            extract_archive(
                &archive,
                &temp.path().join("output"),
                1024,
                &CancelToken::default(),
                &mut |_| {}
            )
            .is_err()
        );
    }

    #[test]
    fn actual_zip_traversal_and_links_cannot_escape_the_destination() {
        let temp = tempfile::tempdir().unwrap();
        for (index, is_link) in [false, true].into_iter().enumerate() {
            let path = temp.path().join(format!("hostile-{index}.zip"));
            let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            if is_link {
                writer.add_symlink("link", "../outside", options).unwrap();
            } else {
                writer.start_file("../outside", options).unwrap();
                writer.write_all(b"hostile").unwrap();
            }
            writer.finish().unwrap();
            let dest = temp.path().join(format!("dest-{index}"));
            assert!(
                extract_archive(
                    &path,
                    &dest,
                    1_048_576,
                    &CancelToken::default(),
                    &mut |_| {}
                )
                .is_err()
            );
            assert!(!temp.path().join("outside").exists());
            assert!(!dest.join("link").exists());
        }
    }

    #[test]
    fn actual_tar_symlink_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("links.tar");
        let mut writer = tar::Builder::new(File::create(&path).unwrap());
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_link_name("../outside").unwrap();
        header.set_cksum();
        writer
            .append_data(&mut header, "link", std::io::empty())
            .unwrap();
        writer.finish().unwrap();
        drop(writer);
        let dest = temp.path().join("dest");
        assert!(
            extract_archive(
                &path,
                &dest,
                1_048_576,
                &CancelToken::default(),
                &mut |_| {}
            )
            .is_err()
        );
        assert!(!dest.join("link").exists());
    }

    #[test]
    fn compression_ratio_limit_applies_even_with_a_large_user_budget() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("zeros");
        std::fs::write(&file, vec![0; 256 * 1024]).unwrap();
        let path = temp.path().join("compressed.zip");
        create_archive(
            &[file],
            &path,
            ArchiveFormat::Zip,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(
            extract_archive(
                &path,
                &temp.path().join("output"),
                u64::MAX,
                &CancelToken::default(),
                &mut |_| {}
            )
            .is_err()
        );
    }

    #[test]
    fn creating_an_archive_can_be_cancelled_mid_stream() {
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("large");
            std::fs::write(&source, vec![42; 2 * 1024 * 1024]).unwrap();
            let output = temp.path().join("cancelled.zip");
            let cancel = CancelToken::default();
            let result =
                create_archive(&[source], &output, ArchiveFormat::Zip, &cancel, &mut |p| {
                    if p.done > 0 {
                        cancel.cancel();
                    }
                });
            send.send((result.unwrap_err().code(), output.exists()))
                .unwrap();
        });
        let (code, exists) = receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("cancelled streaming jobs must return promptly");
        assert_eq!(code, "cancelled");
        assert!(!exists);
    }

    #[test]
    fn zip_metadata_preflight_rejects_excessive_directory_size() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("metadata.zip");
        let mut eocd = [0_u8; 22];
        eocd[..4].copy_from_slice(b"PK\x05\x06");
        eocd[12..16].copy_from_slice(&(128_u32 * 1024 * 1024).to_le_bytes());
        std::fs::write(&path, eocd).unwrap();
        assert!(preflight_zip(&mut File::open(path).unwrap()).is_err());
    }

    fn zip_fixture() -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .start_file("file.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"contents").unwrap();
        writer.finish().unwrap().into_inner()
    }
    fn zip64_fixture(count: u64, extra: usize, offset_only: bool) -> Vec<u8> {
        let original = zip_fixture();
        let end = original.len() - 22;
        let mut footer = original[end..].to_vec();
        let size = u32::from_le_bytes(footer[12..16].try_into().unwrap());
        let offset = u32::from_le_bytes(footer[16..20].try_into().unwrap());
        let mut bytes = original[..end].to_vec();
        let mut record = [0_u8; 56];
        record[..4].copy_from_slice(b"PK\x06\x06");
        record[4..12].copy_from_slice(&(44 + extra as u64).to_le_bytes());
        record[12..14].copy_from_slice(&45_u16.to_le_bytes());
        record[14..16].copy_from_slice(&45_u16.to_le_bytes());
        record[24..32].copy_from_slice(&count.to_le_bytes());
        record[32..40].copy_from_slice(&count.to_le_bytes());
        record[40..48].copy_from_slice(&u64::from(size).to_le_bytes());
        record[48..56].copy_from_slice(&u64::from(offset).to_le_bytes());
        bytes.extend_from_slice(&record);
        bytes.resize(bytes.len() + extra, 0);
        let mut locator = [0_u8; 20];
        locator[..4].copy_from_slice(b"PK\x06\x07");
        locator[8..16].copy_from_slice(&(end as u64).to_le_bytes());
        locator[16..20].copy_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&locator);
        footer[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        if !offset_only {
            footer[8..12].fill(0xff);
            footer[12..16].fill(0xff);
        }
        bytes.extend_from_slice(&footer);
        bytes
    }
    #[test]
    fn zip64_offset_sentinel_cannot_bypass_metadata_budgets() {
        let temp = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("count.zip", zip64_fixture(100_001, 0, true)),
            ("extra.zip", zip64_fixture(1, 1024 * 1024 + 1, true)),
        ] {
            let path = temp.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            assert!(
                preflight_zip(&mut File::open(&path).unwrap()).is_err(),
                "{name}"
            );
        }
    }
    #[test]
    fn zip_rejects_inconsistent_counts_and_noncontiguous_directory_layout() {
        let temp = tempfile::tempdir().unwrap();
        let original = zip_fixture();
        let end = original.len() - 22;
        let mut counts = original.clone();
        counts[end + 8..end + 10].copy_from_slice(&2_u16.to_le_bytes());
        let mut gap = original[..end].to_vec();
        gap.extend_from_slice(b"junk");
        gap.extend_from_slice(&original[end..]);
        for (name, bytes) in [("counts.zip", counts), ("gap.zip", gap)] {
            let path = temp.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            assert!(
                preflight_zip(&mut File::open(path).unwrap()).is_err(),
                "{name}"
            );
        }
    }
    #[test]
    fn valid_zip64_and_local_header_consistency_are_verified() {
        let temp = tempfile::tempdir().unwrap();
        let valid = temp.path().join("valid.zip");
        std::fs::write(&valid, zip64_fixture(1, 0, false)).unwrap();
        let dest = temp.path().join("valid");
        extract_archive(
            &valid,
            &dest,
            1024 * 1024,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(std::fs::read(dest.join("file.txt")).unwrap(), b"contents");
        let mut inconsistent = zip_fixture();
        inconsistent[30] = b'x';
        let input = temp.path().join("inconsistent.zip");
        std::fs::write(&input, inconsistent).unwrap();
        let dest = temp.path().join("inconsistent");
        assert!(
            extract_archive(
                &input,
                &dest,
                1024 * 1024,
                &CancelToken::default(),
                &mut |_| {}
            )
            .is_err()
        );
        assert!(!dest.join("file.txt").exists());
    }

    #[test]
    fn seven_z_extracts_with_the_bounded_decoder() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"seven-z original contents").unwrap();
        let archive = temp.path().join("source.7z");
        sevenz_rust2::compress_to_path(&source, &archive).unwrap();
        let dest = temp.path().join("extracted");
        extract_archive(
            &archive,
            &dest,
            1_048_576,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            std::fs::read(dest.join("source.txt")).unwrap(),
            b"seven-z original contents"
        );
    }
}
