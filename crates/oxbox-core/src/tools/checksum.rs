//! Tool: **Verify Checksum** (`verify-checksum`).
//!
//! Reference implementation of a *file* tool: streams input (never loads a whole file
//! into memory) and is meant to be called from a blocking thread by the Tauri layer.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Algorithm {
    Sha256,
    Sha1,
    Md5,
    Blake3,
}

/// Hash everything readable from `reader` and return lowercase hex.
pub fn hash_reader<R: Read>(mut reader: R, algo: Algorithm) -> CoreResult<String> {
    let mut hasher = Hasher::new(algo);
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(CoreError::Io(e)),
        }
    }
    Ok(hasher.finalize_hex())
}

/// Hash a file in streaming fashion.
pub fn hash_file(path: &Path, algo: Algorithm) -> CoreResult<String> {
    let file = File::open(path)?;
    hash_reader(BufReader::new(file), algo)
}

/// Streaming variant used by the job runner, checking cancellation every 64 KiB.
pub fn hash_file_with_progress(
    path: &Path,
    algo: Algorithm,
    cancel: &crate::jobs::CancelToken,
    progress: &mut dyn FnMut(crate::jobs::Progress),
) -> CoreResult<String> {
    cancel.check()?;
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(CoreError::InvalidInput("choose a regular file".into()));
    }
    let total = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new(algo);
    let mut buffer = [0_u8; 64 * 1024];
    let mut done = 0;
    loop {
        cancel.check()?;
        let count = match reader.read(&mut buffer) {
            Ok(count) => count,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        };
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        done += count as u64;
        progress(crate::jobs::Progress {
            done,
            total,
            message: "Calculating checksum".into(),
        });
    }
    cancel.check()?;
    progress(crate::jobs::Progress {
        done,
        total,
        message: "Checksum complete".into(),
    });
    Ok(hasher.finalize_hex())
}

/// Compare a computed digest with one the user pasted (case- and whitespace-insensitive).
pub fn digests_match(computed: &str, expected: &str) -> bool {
    let normalize = |s: &str| -> String {
        s.chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect()
    };
    let expected = normalize(expected);
    !expected.is_empty() && normalize(computed) == expected
}

enum Hasher {
    Sha256(sha2::Sha256),
    Sha1(sha1::Sha1),
    Md5(md5::Md5),
    Blake3(Box<blake3::Hasher>),
}

impl Hasher {
    fn new(algo: Algorithm) -> Self {
        use sha2::Digest;
        match algo {
            Algorithm::Sha256 => Hasher::Sha256(sha2::Sha256::new()),
            Algorithm::Sha1 => Hasher::Sha1(sha1::Sha1::new()),
            Algorithm::Md5 => Hasher::Md5(md5::Md5::new()),
            Algorithm::Blake3 => Hasher::Blake3(Box::new(blake3::Hasher::new())),
        }
    }

    fn update(&mut self, data: &[u8]) {
        use sha2::Digest;
        match self {
            Hasher::Sha256(h) => h.update(data),
            Hasher::Sha1(h) => h.update(data),
            Hasher::Md5(h) => h.update(data),
            Hasher::Blake3(h) => {
                h.update(data);
            }
        }
    }

    fn finalize_hex(self) -> String {
        use sha2::Digest;
        match self {
            Hasher::Sha256(h) => to_hex(&h.finalize()),
            Hasher::Sha1(h) => to_hex(&h.finalize()),
            Hasher::Md5(h) => to_hex(&h.finalize()),
            Hasher::Blake3(h) => h.finalize().to_hex().to_string(),
        }
    }
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // Writing to a String cannot fail.
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Known-answer vectors for "abc" (FIPS 180-2, RFC 1321, BLAKE3 spec).
    const ABC: &[u8] = b"abc";

    #[test]
    fn progress_is_monotonic_and_can_cancel_during_hashing() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&vec![42; 200_000]).unwrap();
        let token = crate::jobs::CancelToken::default();
        let mut done = 0;
        let result = hash_file_with_progress(f.path(), Algorithm::Blake3, &token, &mut |p| {
            assert!(p.done >= done);
            done = p.done;
            if done >= 65_536 {
                token.cancel();
            }
        });
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert!(done < 200_000);
    }

    #[test]
    fn known_vectors() {
        let cases = [
            (
                Algorithm::Sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (Algorithm::Sha1, "a9993e364706816aba3e25717850c26c9cd0d89d"),
            (Algorithm::Md5, "900150983cd24fb0d6963f7d28e17f72"),
            (
                Algorithm::Blake3,
                "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
            ),
        ];
        for (algo, want) in cases {
            assert_eq!(hash_reader(ABC, algo).unwrap(), want, "{algo:?}");
        }
    }

    #[test]
    fn hash_file_matches_hash_reader() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(ABC).unwrap();
        let from_file = hash_file(f.path(), Algorithm::Sha256).unwrap();
        assert_eq!(from_file, hash_reader(ABC, Algorithm::Sha256).unwrap());
    }

    #[test]
    fn large_input_streams() {
        // 5 MiB of zeros through a reader that is not a file.
        let data = std::io::repeat(0).take(5 * 1024 * 1024);
        let digest = hash_reader(data, Algorithm::Blake3).unwrap();
        assert_eq!(digest.len(), 64);
    }

    #[test]
    fn missing_file_is_not_found() {
        let err = hash_file(Path::new("definitely/not/here.bin"), Algorithm::Md5).unwrap_err();
        assert_eq!(err.code(), "not_found");
    }

    #[test]
    fn digests_match_ignores_case_and_whitespace() {
        assert!(digests_match("abcdef", "  ABCDEF\n"));
        assert!(!digests_match("abcdef", "abcdee"));
        assert!(!digests_match("abcdef", ""));
    }

    #[test]
    fn algorithm_uses_lowercase_on_the_wire() {
        assert_eq!(
            serde_json::to_string(&Algorithm::Blake3).unwrap(),
            "\"blake3\""
        );
    }
}
