//! Streaming age passphrase encryption; originals are always preserved.

use std::cell::{Cell, RefCell};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use age::secrecy::{ExposeSecret, SecretString};

use crate::jobs::{CancelToken, Progress};
use crate::{CoreError, CoreResult};

fn crypt_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("file encryption failed: {error}"))
}

const MAX_HEADER_BYTES: usize = 64 * 1024;
// age uses scrypt r=8,p=1: its main allocation is approximately 1024 * 2^log_n
// bytes, plus implementation overhead. Avoid CPU-calibrated unbounded RAM use.
const ENCRYPT_WORK_FACTOR: u8 = 18;
const MAX_DECRYPT_WORK_FACTOR: u8 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
    identity: (u64, u64),
}
impl FileStamp {
    fn read(file: &File) -> CoreResult<Self> {
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(crypt_error("choose a regular file"));
        }
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            identity: file_identity(file, &metadata)?,
        })
    }
}
fn file_identity(file: &File, metadata: &fs::Metadata) -> CoreResult<(u64, u64)> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let _ = metadata;
        let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut information) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok((
            u64::from(information.dwVolumeSerialNumber),
            (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow),
        ))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = file;
        Ok((metadata.dev(), metadata.ino()))
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (file, metadata);
        Err(crypt_error("file identity is unavailable on this platform"))
    }
}

/// Transient proof of the bytes processed and output written by one successful job.
/// No passphrase is retained, and the receipt is deliberately not serializable.
#[derive(Debug, Clone)]
pub struct TransformReceipt {
    pub source: PathBuf,
    pub output: PathBuf,
    source_stamp: FileStamp,
    source_hash: [u8; 32],
    output_stamp: FileStamp,
    output_hash: [u8; 32],
}
struct DigestReader<R> {
    input: R,
    digest: Rc<RefCell<blake3::Hasher>>,
}
impl<R: Read> Read for DigestReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = self.input.read(bytes)?;
        self.digest.borrow_mut().update(&bytes[..count]);
        Ok(count)
    }
}
struct DigestWriter<W> {
    output: W,
    digest: Rc<RefCell<blake3::Hasher>>,
}
impl<W: Write> Write for DigestWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = self.output.write(bytes)?;
        self.digest.borrow_mut().update(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()
    }
}

fn changed_receipt() -> CoreError {
    crypt_error(
        "the original or saved output is missing, replaced or changed; keep both files and process/review them again before Trash",
    )
}
fn capture_fingerprint(
    path: &Path,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<(FileStamp, [u8; 32])> {
    cancel.check()?;
    crate::fs_safe::reject_symlink_components(path)?;
    let mut file = File::open(path)?;
    let before = FileStamp::read(&file)?;
    let mut hash = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut done = 0_u64;
    loop {
        cancel.check()?;
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        done = done.saturating_add(count as u64);
        if done > before.len {
            return Err(changed_receipt());
        }
        hash.update(&buffer[..count]);
        progress(Progress {
            done,
            total: before.len,
            message: "Revalidating original and saved output before Trash".into(),
        });
    }
    cancel.check()?;
    if done != before.len || FileStamp::read(&file)? != before {
        return Err(changed_receipt());
    }
    Ok((before, *hash.finalize().as_bytes()))
}
fn validate_stamp(path: &Path, expected: &FileStamp) -> CoreResult<()> {
    crate::fs_safe::reject_symlink_components(path)?;
    if FileStamp::read(&File::open(path)?)? != *expected {
        return Err(changed_receipt());
    }
    Ok(())
}

/// Revalidate both processed bytes and saved backup before an explicitly confirmed Trash request.
/// This preflight narrows ordinary replacement/edit races; it is not an atomic filesystem transaction.
pub fn trash_original(
    receipt: &TransformReceipt,
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<usize> {
    trash_original_with(
        receipt,
        confirmed,
        cancel,
        progress,
        crate::fs_safe::move_to_trash,
    )
}
fn trash_original_with(
    receipt: &TransformReceipt,
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    trash: impl FnOnce(&[PathBuf]) -> CoreResult<usize>,
) -> CoreResult<usize> {
    cancel.check()?;
    if !confirmed {
        return Err(crypt_error("confirm the reviewed original before Trash"));
    }
    let source = crate::fs_safe::validate_trash_path(&receipt.source)?;
    for (path, stamp, hash) in [
        (&receipt.source, &receipt.source_stamp, &receipt.source_hash),
        (&receipt.output, &receipt.output_stamp, &receipt.output_hash),
    ] {
        cancel.check()?;
        validate_stamp(path, stamp).map_err(|_| changed_receipt())?;
        let current = capture_fingerprint(path, cancel, progress).map_err(|error| {
            if matches!(error, CoreError::Cancelled) {
                error
            } else {
                changed_receipt()
            }
        })?;
        if current.0 != *stamp || current.1 != *hash {
            return Err(changed_receipt());
        }
    }
    // Recheck earlier identities/stamps after both full streaming comparisons.
    // Files must remain closed/unchanged; another process can still race the OS Trash call.
    validate_stamp(&receipt.source, &receipt.source_stamp).map_err(|_| changed_receipt())?;
    validate_stamp(&receipt.output, &receipt.output_stamp).map_err(|_| changed_receipt())?;
    cancel.check()?;
    trash(&[source])
}

/// Limit age's accumulating header parser, then lift only that budget for the
/// streaming encrypted payload. Cancellation remains active for all reads.
struct HeaderReader<R> {
    input: R,
    header: Rc<Cell<bool>>,
    remaining: usize,
    cancel: CancelToken,
}
impl<R: BufRead> Read for HeaderReader<R> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        let available = self.fill_buf()?;
        let count = available.len().min(output.len());
        output[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}
impl<R: BufRead> BufRead for HeaderReader<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.cancel
            .check()
            .map_err(|_| std::io::Error::other("Cancelled"))?;
        if self.header.get() && self.remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "age header exceeds 64 KiB",
            ));
        }
        let buffer = self.input.fill_buf()?;
        if self.header.get() {
            Ok(&buffer[..buffer.len().min(self.remaining)])
        } else {
            Ok(buffer)
        }
    }
    fn consume(&mut self, count: usize) {
        self.input.consume(count);
        if self.header.get() {
            self.remaining = self.remaining.saturating_sub(count);
        }
    }
}

fn bounded_decryptor<R: BufRead>(
    input: R,
    cancel: &CancelToken,
) -> CoreResult<age::Decryptor<HeaderReader<R>>> {
    cancel.check()?;
    let header = Rc::new(Cell::new(true));
    let result = age::Decryptor::new_buffered(HeaderReader {
        input,
        header: header.clone(),
        remaining: MAX_HEADER_BYTES,
        cancel: cancel.clone(),
    });
    header.set(false);
    cancel.check()?;
    result.map_err(|_| {
        CoreError::InvalidInput("the age file is invalid or its header exceeds 64 KiB".into())
    })
}

fn stream_copy(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    total: u64,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<()> {
    let mut buffer = [0_u8; 64 * 1024];
    let mut done = 0;
    loop {
        cancel.check()?;
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        writer.write_all(&buffer[..count])?;
        done += count as u64;
        progress(Progress {
            done: done.min(total),
            total,
            message: "Processing file".into(),
        });
    }
    cancel.check()?;
    Ok(())
}

/// The passphrase is moved into a zeroizing SecretString and is never included in errors.
/// A create-new output is removed on error/cancellation; source files are never modified.
/// New files use age scrypt factor 18; decryption refuses factors above 20.
/// Higher-factor legacy age inputs remain intact but require another compatible tool.
pub fn transform_file(
    source: &Path,
    output: &Path,
    passphrase: String,
    decrypt: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<()> {
    transform_file_with_receipt(source, output, passphrase, decrypt, cancel, progress).map(|_| ())
}

pub fn transform_file_with_receipt(
    source: &Path,
    output: &Path,
    passphrase: String,
    decrypt: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<TransformReceipt> {
    let passphrase = SecretString::from(passphrase);
    if passphrase.expose_secret().is_empty() || passphrase.expose_secret().len() > 4096 {
        return Err(CoreError::InvalidInput(
            "enter a passphrase of 1–4096 bytes".into(),
        ));
    }
    cancel.check()?;
    let input = File::open(source)?;
    let source_stamp = FileStamp::read(&input)?;
    let total = source_stamp.len;
    let source_hash = Rc::new(RefCell::new(blake3::Hasher::new()));
    let output_hash = Rc::new(RefCell::new(blake3::Hasher::new()));
    let mut input = BufReader::new(DigestReader {
        input,
        digest: source_hash.clone(),
    });
    let source_path = std::path::absolute(source)?;
    // Resolve the parent before creating output; never replace an existing destination.
    let parent = output
        .parent()
        .ok_or_else(|| CoreError::InvalidInput("choose an output filename".into()))?
        .canonicalize()?;
    let filename = output
        .file_name()
        .ok_or_else(|| CoreError::InvalidInput("choose an output filename".into()))?;
    crate::fs_safe::validate_output_filename(filename)?;
    let destination = parent.join(filename);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&destination)?;
    if !file.metadata()?.is_file() {
        return Err(CoreError::InvalidInput(
            "the output must be a regular file".into(),
        ));
    }
    let result = (|| {
        let mut writer = DigestWriter {
            output: BufWriter::new(file),
            digest: output_hash.clone(),
        };
        if decrypt {
            let decryptor = bounded_decryptor(input, cancel)?;
            if !decryptor.is_scrypt() {
                return Err(CoreError::InvalidInput(
                    "this file requires an age identity rather than a passphrase".into(),
                ));
            }
            let mut identity = age::scrypt::Identity::new(passphrase);
            identity.set_max_work_factor(MAX_DECRYPT_WORK_FACTOR);
            let mut reader = decryptor
                .decrypt(std::iter::once(&identity as &dyn age::Identity))
                .map_err(|error| match error {
                    age::DecryptError::ExcessiveWork { .. } => CoreError::InvalidInput(format!(
                        "the age scrypt work factor exceeds Oxbox's safe limit ({MAX_DECRYPT_WORK_FACTOR}; approximately 1 GiB plus overhead); the original is preserved"
                    )),
                    _ => CoreError::InvalidInput(
                        "the passphrase is incorrect or the encrypted file is damaged".into(),
                    ),
                })?;
            stream_copy(&mut reader, &mut writer, total, cancel, progress)?;
        } else {
            cancel.check()?;
            let mut recipient = age::scrypt::Recipient::new(passphrase);
            recipient.set_work_factor(ENCRYPT_WORK_FACTOR);
            cancel.check()?;
            let encryptor =
                age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
                    .map_err(crypt_error)?;
            cancel.check()?;
            let mut encrypted = encryptor.wrap_output(&mut writer).map_err(crypt_error)?;
            stream_copy(&mut input, &mut encrypted, total, cancel, progress)?;
            encrypted.finish().map_err(crypt_error)?;
        }
        writer.flush()?;
        writer.output.get_ref().sync_all()?;
        let written_stamp = FileStamp::read(writer.output.get_ref())?;
        drop(writer);
        // Closing a Windows write handle can finalize its last-write timestamp.
        // Retain the original identity if an external actor replaced the output path.
        let current_stamp = FileStamp::read(&File::open(&destination)?)?;
        let output_stamp = if current_stamp.identity == written_stamp.identity {
            current_stamp
        } else {
            written_stamp
        };
        cancel.check()?;
        progress(Progress {
            done: total,
            total,
            message: if decrypt {
                "File decrypted"
            } else {
                "File encrypted"
            }
            .into(),
        });
        Ok(TransformReceipt {
            source: source_path,
            output: destination.clone(),
            source_stamp,
            source_hash: *source_hash.borrow().finalize().as_bytes(),
            output_stamp,
            output_hash: *output_hash.borrow().finalize().as_bytes(),
        })
    })();
    let result = if result.is_err() && cancel.check().is_err() {
        Err(CoreError::Cancelled)
    } else {
        result
    };
    if result.is_err()
        && let Err(cleanup) = fs::remove_file(&destination)
    {
        return Err(CoreError::InvalidInput(format!(
            "processing failed and the incomplete output could not be removed: {} ({cleanup})",
            destination.display()
        )));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_AGE_0115: &[u8] = include_bytes!("../../tests/fixtures/age-0.11.5/legacy.age");
    const LEGACY_PLAINTEXT: &[u8] = include_bytes!("../../tests/fixtures/age-0.11.5/plaintext.bin");
    const LEGACY_FIXTURE_PASSPHRASE: &str = "Oxbox public age 0.11.5 fixture password";

    #[test]
    fn legacy_age_0115_generated_fixture_decrypts_without_changing_input() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("generated-old.age");
        let restored = directory.path().join("restored.bin");
        fs::write(&source, LEGACY_AGE_0115).unwrap();
        assert!(LEGACY_AGE_0115.starts_with(b"age-encryption.org/v1\n-> scrypt "));
        let recipient = LEGACY_AGE_0115.split(|byte| *byte == b'\n').nth(1).unwrap();
        assert!(recipient.ends_with(b" 10"));
        assert!(LEGACY_PLAINTEXT.contains(&0) && LEGACY_PLAINTEXT.contains(&255));
        transform_file(
            &source,
            &restored,
            LEGACY_FIXTURE_PASSPHRASE.into(),
            true,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(fs::read(restored).unwrap(), LEGACY_PLAINTEXT);
        assert_eq!(fs::read(source).unwrap(), LEGACY_AGE_0115);
    }

    #[test]
    fn legacy_age_0115_fixture_keeps_error_cancel_and_exclusive_output_guards() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("generated-old.age");
        let output = directory.path().join("must-not-remain.bin");
        fs::write(&source, LEGACY_AGE_0115).unwrap();
        let wrong_password = transform_file(
            &source,
            &output,
            "generated wrong fixture password".into(),
            true,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap_err();
        assert_eq!(wrong_password.code(), "invalid_input");
        assert!(
            !wrong_password
                .to_string()
                .contains(LEGACY_FIXTURE_PASSPHRASE)
        );
        assert!(!output.exists());

        let mut corrupt = LEGACY_AGE_0115.to_vec();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        let damaged = directory.path().join("generated-corrupt.age");
        fs::write(&damaged, &corrupt).unwrap();
        assert!(
            transform_file(
                &damaged,
                &output,
                LEGACY_FIXTURE_PASSPHRASE.into(),
                true,
                &CancelToken::default(),
                &mut |_| {},
            )
            .is_err()
        );
        assert!(!output.exists());

        let cancel = CancelToken::default();
        let mut progressed = false;
        let cancelled = transform_file(
            &source,
            &output,
            LEGACY_FIXTURE_PASSPHRASE.into(),
            true,
            &cancel,
            &mut |progress| {
                if progress.done > 0 {
                    progressed = true;
                    cancel.cancel();
                }
            },
        )
        .unwrap_err();
        assert!(
            progressed,
            "cancel test must reach actual decryption progress"
        );
        assert_eq!(cancelled.code(), "cancelled");
        assert!(!output.exists());

        fs::write(&output, b"preserve occupied generated destination").unwrap();
        assert!(
            transform_file(
                &source,
                &output,
                LEGACY_FIXTURE_PASSPHRASE.into(),
                true,
                &CancelToken::default(),
                &mut |_| {},
            )
            .is_err()
        );
        assert_eq!(
            fs::read(output).unwrap(),
            b"preserve occupied generated destination"
        );
        assert_eq!(fs::read(source).unwrap(), LEGACY_AGE_0115);
    }

    fn fixture_receipt(source: &Path, output: &Path) -> TransformReceipt {
        let token = CancelToken::default();
        let (source_stamp, source_hash) = capture_fingerprint(source, &token, &mut |_| {}).unwrap();
        let (output_stamp, output_hash) = capture_fingerprint(output, &token, &mut |_| {}).unwrap();
        TransformReceipt {
            source: source.to_path_buf(),
            output: output.to_path_buf(),
            source_stamp,
            source_hash,
            output_stamp,
            output_hash,
        }
    }

    #[test]
    fn encryption_trash_receipt_refuses_changed_original_or_missing_changed_backup() {
        for changed in ["source", "output", "missing-output", "replacement-source"] {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("source.txt");
            let output = directory.path().join("backup.age");
            fs::write(&source, b"source fixture").unwrap();
            fs::write(&output, b"output fixture").unwrap();
            let receipt = fixture_receipt(&source, &output);
            let held_original = File::open(&source).unwrap();
            match changed {
                "source" => fs::write(&source, b"edited source!").unwrap(),
                "output" => fs::write(&output, b"edited backup!").unwrap(),
                "missing-output" => fs::remove_file(&output).unwrap(),
                _ => {
                    fs::remove_file(&source).unwrap();
                    fs::write(&source, b"source fixture").unwrap();
                    File::options()
                        .write(true)
                        .open(&source)
                        .unwrap()
                        .set_modified(receipt.source_stamp.modified.unwrap())
                        .unwrap();
                }
            }
            let mut effected = false;
            let result =
                trash_original_with(&receipt, true, &CancelToken::default(), &mut |_| {}, |_| {
                    effected = true;
                    Ok(1)
                });
            assert!(
                result.is_err(),
                "{changed} must invalidate the backup receipt"
            );
            assert!(!effected, "changed files must not reach the Trash backend");
            assert!(source.exists());
            drop(held_original);
        }
    }

    #[test]
    fn encryption_trash_receipt_requires_confirmation_and_cancellation_preflight() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.txt");
        let output = directory.path().join("backup.age");
        fs::write(&source, b"source fixture").unwrap();
        fs::write(&output, b"output fixture").unwrap();
        let receipt = fixture_receipt(&source, &output);
        let mut calls = 0;
        assert_eq!(
            trash_original_with(
                &receipt,
                true,
                &CancelToken::default(),
                &mut |_| {},
                |paths| {
                    calls += 1;
                    assert_eq!(paths, &[source.canonicalize().unwrap()]);
                    assert!(output.exists());
                    Ok(1)
                }
            )
            .unwrap(),
            1
        );
        assert_eq!(calls, 1);
        assert!(
            trash_original_with(
                &receipt,
                false,
                &CancelToken::default(),
                &mut |_| {},
                |_| panic!("unconfirmed Trash")
            )
            .is_err()
        );
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            trash_original_with(&receipt, true, &cancel, &mut |_| {}, |_| panic!(
                "cancelled Trash"
            ))
            .unwrap_err()
            .code(),
            "cancelled"
        );
    }

    #[test]
    fn encryption_receipt_hashes_actual_stream_when_source_rewrites_preserve_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.txt");
        let output = directory.path().join("backup.fixture");
        fs::write(&source, vec![b'a'; 128 * 1024]).unwrap();
        let input_file = File::open(&source).unwrap();
        let source_stamp = FileStamp::read(&input_file).unwrap();
        let source_digest = Rc::new(std::cell::RefCell::new(blake3::Hasher::new()));
        let output_digest = Rc::new(std::cell::RefCell::new(blake3::Hasher::new()));
        let mut reader = DigestReader {
            input: input_file,
            digest: source_digest.clone(),
        };
        let mut writer = DigestWriter {
            output: File::create(&output).unwrap(),
            digest: output_digest.clone(),
        };
        let mut changed = false;
        stream_copy(
            &mut reader,
            &mut writer,
            source_stamp.len,
            &CancelToken::default(),
            &mut |_| {
                if !changed {
                    changed = true;
                    let mut edit = File::options().write(true).open(&source).unwrap();
                    edit.write_all(b"changed already-read prefix").unwrap();
                    edit.set_modified(source_stamp.modified.unwrap()).unwrap();
                }
            },
        )
        .unwrap();
        writer.flush().unwrap();
        let receipt = TransformReceipt {
            source: source.clone(),
            output: output.clone(),
            source_stamp,
            source_hash: *source_digest.borrow().finalize().as_bytes(),
            output_stamp: FileStamp::read(&writer.output).unwrap(),
            output_hash: *output_digest.borrow().finalize().as_bytes(),
        };
        assert!(
            trash_original_with(
                &receipt,
                true,
                &CancelToken::default(),
                &mut |_| {},
                |_| panic!("mixed snapshot Trash")
            )
            .is_err()
        );
        assert_ne!(fs::read(source).unwrap(), fs::read(output).unwrap());
    }
    #[test]
    fn privacy_excessive_work_is_distinct_without_running_an_expensive_kdf() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("generated-excessive.age");
        let output = directory.path().join("must-not-leave-plaintext");
        // Canonical 16-byte salt / 32-byte body and MAC encodings. Factor 99
        // exceeds even age's maximum possible default (target <64, plus4), so
        // the baseline also rejects it before scrypt. Only error policy changes.
        let mut fixture = b"age-encryption.org/v1\n-> scrypt AAAAAAAAAAAAAAAAAAAAAA 99\nAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n--- AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n".to_vec();
        // Decryptor construction also consumes the 16-byte payload nonce.
        fixture.extend_from_slice(&[0; 16]);
        assert!(
            bounded_decryptor(std::io::Cursor::new(&fixture), &CancelToken::default())
                .unwrap()
                .is_scrypt()
        );
        std::fs::write(&source, &fixture).unwrap();
        let passphrase = "generated privacy passphrase sentinel";
        let error = transform_file(
            &source,
            &output,
            passphrase.into(),
            true,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("work factor exceeds Oxbox's safe limit"),
            "{error}"
        );
        assert!(!error.to_string().contains(passphrase));
        assert!(!output.exists());
        assert_eq!(std::fs::read(source).unwrap(), fixture);
    }

    #[test]
    fn release_media_bounds_reject_oversized_age_header_before_reading_it_all() {
        let mut bytes = b"age-encryption.org/v1\n-> scrypt ".to_vec();
        bytes.extend(vec![b'A'; 128 * 1024]);
        bytes.push(b'\n');
        let mut input = std::io::Cursor::new(bytes);
        assert!(bounded_decryptor(&mut input, &CancelToken::default()).is_err());
        assert!(
            input.position() <= 64 * 1024,
            "header parsing consumed {} bytes",
            input.position()
        );
    }
    #[cfg(windows)]
    #[test]
    fn encryption_refuses_ntfs_stream_outputs_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.bin");
        let base = dir.path().join("existing.bin");
        std::fs::write(&source, b"source fixture").unwrap();
        std::fs::write(&base, b"existing fixture").unwrap();
        let error = transform_file(
            &source,
            &dir.path().join("existing.bin:output.age"),
            "fixture passphrase".into(),
            false,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap_err();
        assert_eq!(error.code(), "invalid_input");
        assert_eq!(std::fs::read(base).unwrap(), b"existing fixture");
    }

    #[test]
    fn round_trip_wrong_password_and_no_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let encrypted = dir.path().join("source.age");
        let decrypted = dir.path().join("restored");
        std::fs::write(&source, b"original contents\0with binary".repeat(5000)).unwrap();
        let token = crate::jobs::CancelToken::default();
        let receipt = transform_file_with_receipt(
            &source,
            &encrypted,
            "correct horse battery staple".into(),
            false,
            &token,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            receipt.source_hash,
            *blake3::hash(&fs::read(&source).unwrap()).as_bytes()
        );
        assert_eq!(
            receipt.output_hash,
            *blake3::hash(&fs::read(&encrypted).unwrap()).as_bytes()
        );
        assert_eq!(
            trash_original_with(&receipt, true, &token, &mut |_| {}, |_| Ok(1)).unwrap(),
            1
        );
        let original = fs::read(&source).unwrap();
        fs::write(&source, b"edited original after encryption").unwrap();
        assert!(
            trash_original_with(&receipt, true, &token, &mut |_| {}, |_| panic!(
                "stale real encryption receipt reached Trash"
            ))
            .is_err()
        );
        fs::write(&source, original).unwrap();
        assert!(source.exists());
        let mut header = BufReader::new(File::open(&encrypted).unwrap()).lines();
        assert_eq!(header.next().unwrap().unwrap(), "age-encryption.org/v1");
        let recipient = header.next().unwrap().unwrap();
        assert!(recipient.starts_with("-> scrypt "));
        assert!(recipient.ends_with(" 18"), "{recipient}");
        assert!(
            transform_file(
                &encrypted,
                &decrypted,
                "wrong passphrase".into(),
                true,
                &token,
                &mut |_| {}
            )
            .is_err()
        );
        // Damage authenticated ciphertext after a valid header: no partial plaintext remains.
        let damaged = dir.path().join("damaged.age");
        let damaged_output = dir.path().join("damaged-output");
        let mut bytes = std::fs::read(&encrypted).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(&damaged, &bytes).unwrap();
        assert!(
            transform_file(
                &damaged,
                &damaged_output,
                "correct horse battery staple".into(),
                true,
                &token,
                &mut |_| {}
            )
            .is_err()
        );
        assert!(!damaged_output.exists());
        // A truncated file also must fail without leaving a new plaintext file.
        std::fs::write(&damaged, &bytes[..bytes.len() - 8]).unwrap();
        assert!(
            transform_file(
                &damaged,
                &damaged_output,
                "correct horse battery staple".into(),
                true,
                &token,
                &mut |_| {}
            )
            .is_err()
        );
        assert!(!damaged_output.exists());
        assert!(!decrypted.exists());
        transform_file(
            &encrypted,
            &decrypted,
            "correct horse battery staple".into(),
            true,
            &token,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(&decrypted).unwrap()
        );
        assert!(
            transform_file(
                &source,
                &encrypted,
                "correct horse battery staple".into(),
                false,
                &token,
                &mut |_| {}
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_empty_passphrase_and_cancelled_work() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let output = dir.path().join("out");
        std::fs::write(&source, b"data").unwrap();
        let token = crate::jobs::CancelToken::default();
        assert!(
            transform_file(&source, &output, String::new(), false, &token, &mut |_| {}).is_err()
        );
        token.cancel();
        assert!(
            transform_file(
                &source,
                &output,
                "passphrase".into(),
                false,
                &token,
                &mut |_| {}
            )
            .is_err()
        );
        assert!(!output.exists());
    }
}
