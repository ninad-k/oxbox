//! Streaming age passphrase encryption; originals are always preserved.

use std::cell::Cell;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::rc::Rc;

use age::secrecy::{ExposeSecret, SecretString};

use crate::jobs::{CancelToken, Progress};
use crate::{CoreError, CoreResult};

fn crypt_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("file encryption failed: {error}"))
}

const MAX_HEADER_BYTES: usize = 64 * 1024;

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
pub fn transform_file(
    source: &Path,
    output: &Path,
    passphrase: String,
    decrypt: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<()> {
    let passphrase = SecretString::from(passphrase);
    if passphrase.expose_secret().is_empty() || passphrase.expose_secret().len() > 4096 {
        return Err(CoreError::InvalidInput(
            "enter a passphrase of 1–4096 bytes".into(),
        ));
    }
    cancel.check()?;
    let input = File::open(source)?;
    let metadata = input.metadata()?;
    if !metadata.is_file() {
        return Err(CoreError::InvalidInput("choose a regular file".into()));
    }
    let total = metadata.len();
    let mut input = BufReader::new(input);
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
        let mut writer = BufWriter::new(file);
        if decrypt {
            let decryptor = bounded_decryptor(input, cancel)?;
            if !decryptor.is_scrypt() {
                return Err(CoreError::InvalidInput(
                    "this file requires an age identity rather than a passphrase".into(),
                ));
            }
            let identity = age::scrypt::Identity::new(passphrase);
            let mut reader = decryptor
                .decrypt(std::iter::once(&identity as &dyn age::Identity))
                .map_err(|_| {
                    CoreError::InvalidInput(
                        "the passphrase is incorrect or the encrypted file is damaged".into(),
                    )
                })?;
            stream_copy(&mut reader, &mut writer, total, cancel, progress)?;
        } else {
            let encryptor = age::Encryptor::with_user_passphrase(passphrase);
            cancel.check()?;
            let mut encrypted = encryptor.wrap_output(&mut writer).map_err(crypt_error)?;
            stream_copy(&mut input, &mut encrypted, total, cancel, progress)?;
            encrypted.finish().map_err(crypt_error)?;
        }
        writer.flush()?;
        writer.get_ref().sync_all()?;
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
        Ok(())
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
        transform_file(
            &source,
            &encrypted,
            "correct horse battery staple".into(),
            false,
            &token,
            &mut |_| {},
        )
        .unwrap();
        assert!(source.exists());
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
