//! Resource-policy preflight for 7z metadata and decoder graphs.
use crate::{CoreError, CoreResult, jobs::CancelToken};
use sevenz_rust2::{Archive, ArchiveReader, EncoderMethod as M, Password};
use std::io::{Read, Seek, SeekFrom};

const MAX_TOC: u64 = 128 * 1024;
const MAX_MODELS: u64 = 128 * 1024 * 1024;
fn error(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("archive: 7z {message}"))
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(bytes);
    crc.sum()
}
fn number(bytes: &[u8]) -> u64 {
    let mut value = [0; 8];
    value.copy_from_slice(bytes);
    u64::from_le_bytes(value)
}

fn validate_models<'a>(models: impl Iterator<Item = (&'a [u8], &'a [u8])>) -> CoreResult<()> {
    let mut total = 0_u64;
    for (index, (method, properties)) in models.enumerate() {
        if index >= 4 {
            return Err(error(
                "compression graphs are limited to four coders per block",
            ));
        }
        let bytes = if method == M::ID_LZMA2 {
            if properties.len() != 1 || properties[0] > 40 {
                return Err(error("invalid LZMA2 dictionary properties"));
            }
            let property = u32::from(properties[0]);
            let dictionary = if property == 40 {
                u64::from(u32::MAX)
            } else {
                u64::from(2 | (property & 1)) << (property / 2 + 11)
            };
            dictionary + 2 * 1024 * 1024
        } else if method == M::ID_LZMA {
            if properties.len() != 5
                || properties[0] >= 225
                || properties[0] % 9 + (properties[0] / 9) % 5 > 4
            {
                return Err(error("unsupported LZMA model properties"));
            }
            u64::from(u32::from_le_bytes(
                properties[1..5]
                    .try_into()
                    .map_err(|_| error("invalid LZMA dictionary"))?,
            ))
            .max(4096)
                + 2 * 1024 * 1024
        } else if method == M::ID_PPMD {
            if properties.len() != 5 {
                return Err(error("invalid PPMd model properties"));
            }
            u64::from(u32::from_le_bytes(
                properties[1..5]
                    .try_into()
                    .map_err(|_| error("invalid PPMd model"))?,
            )) + 2 * 1024 * 1024
        } else if method == M::ID_BZIP2 {
            16 * 1024 * 1024
        } else if [
            M::ID_COPY,
            M::ID_DELTA,
            M::ID_BCJ_X86,
            M::ID_BCJ_ARM,
            M::ID_BCJ_ARM64,
            M::ID_BCJ_ARM_THUMB,
            M::ID_BCJ_PPC,
            M::ID_BCJ_IA64,
            M::ID_BCJ_SPARC,
            M::ID_BCJ_RISCV,
        ]
        .contains(&method)
        {
            64 * 1024
        } else {
            return Err(error(
                "compression method or multi-input graph is outside the supported resource profile",
            ));
        };
        total = total.saturating_add(bytes);
        if total > MAX_MODELS {
            return Err(error(
                "combined decoder models exceed the 128 MiB block budget",
            ));
        }
    }
    Ok(())
}
pub(super) fn validate_blocks(archive: &Archive) -> CoreResult<()> {
    for block in &archive.blocks {
        validate_models(
            block
                .coders
                .iter()
                .map(|coder| (coder.encoder_method_id(), coder.properties())),
        )?;
    }
    Ok(())
}

/// A seekable, read-only virtual archive. Packed bytes keep their original
/// offsets; only the 32-byte envelope is replaced and a small TOC is appended.
/// No original data is copied into memory or written to disk.
pub(super) struct Probe<R> {
    source: R,
    length: u64,
    prefix: [u8; 32],
    footer: Vec<u8>,
    position: u64,
}
impl<R: Read + Seek> Probe<R> {
    fn new(source: R, length: u64, mut prefix: [u8; 32], footer: Vec<u8>) -> CoreResult<Self> {
        let offset = length
            .checked_sub(32)
            .ok_or_else(|| error("archive header is truncated"))?;
        length
            .checked_add(footer.len() as u64)
            .ok_or_else(|| error("metadata position overflow"))?;
        prefix[12..20].copy_from_slice(&offset.to_le_bytes());
        prefix[20..28].copy_from_slice(&(footer.len() as u64).to_le_bytes());
        prefix[28..32].copy_from_slice(&crc32(&footer).to_le_bytes());
        let checksum = crc32(&prefix[12..]);
        prefix[8..12].copy_from_slice(&checksum.to_le_bytes());
        Ok(Self {
            source,
            length,
            prefix,
            footer,
            position: 0,
        })
    }
}
impl<R: Read + Seek> Read for Probe<R> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let count = if self.position < 32 {
            let start = self.position as usize;
            let count = out.len().min(32 - start);
            out[..count].copy_from_slice(&self.prefix[start..start + count]);
            count
        } else if self.position < self.length {
            self.source.seek(SeekFrom::Start(self.position))?;
            let count = out
                .len()
                .min((self.length - self.position).min(usize::MAX as u64) as usize);
            let count = self.source.read(&mut out[..count])?;
            if count == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
            }
            count
        } else {
            let offset = usize::try_from(self.position - self.length)
                .map_err(|_| std::io::Error::other("metadata position overflow"))?;
            if offset >= self.footer.len() {
                return Ok(0);
            }
            let count = out.len().min(self.footer.len() - offset);
            out[..count].copy_from_slice(&self.footer[offset..offset + count]);
            count
        };
        self.position += count as u64;
        Ok(count)
    }
}
impl<R: Read + Seek> Seek for Probe<R> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let end = self.length + self.footer.len() as u64;
        let next = match from {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::Current(delta) => i128::from(self.position) + i128::from(delta),
            SeekFrom::End(delta) => i128::from(end) + i128::from(delta),
        };
        if next < 0 || next > i128::from(end) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "7z probe seek is outside the source",
            ));
        }
        self.position = next as u64;
        Ok(self.position)
    }
}

#[derive(Debug)]
pub(super) struct Metadata {
    length: u64,
    prefix: [u8; 32],
    toc: Vec<u8>,
}
impl Metadata {
    pub(super) fn reader<R: Read + Seek>(
        self,
        source: R,
        cancel: &CancelToken,
    ) -> CoreResult<ArchiveReader<Probe<R>>> {
        cancel.check()?;
        // Parse the validated snapshot, not the original encoded header again.
        // Concurrent changes to the source cannot replace its decoder graph.
        let probe = Probe::new(source, self.length, self.prefix, self.toc)?;
        let mut reader = ArchiveReader::new_with_memory_limit(probe, Password::empty(), 128 * 1024)
            .map_err(|e| error(&e.to_string()))?;
        reader.set_thread_count(1);
        if reader.archive().files.len() > 100_000 {
            return Err(error("archive has more than 100,000 entries"));
        }
        validate_blocks(reader.archive())?;
        cancel.check()?;
        Ok(reader)
    }
}

pub(super) fn preflight<R: Read + Seek>(
    source: &mut R,
    cancel: &CancelToken,
) -> CoreResult<Metadata> {
    cancel.check()?;
    let length = source.seek(SeekFrom::End(0))?;
    source.seek(SeekFrom::Start(0))?;
    let mut prefix = [0; 32];
    source.read_exact(&mut prefix)?;
    if prefix[..6] != [b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c] || prefix[6] != 0 {
        return Err(error("signature or version is unsupported"));
    }
    if crc32(&prefix[12..])
        != u32::from_le_bytes(
            prefix[8..12]
                .try_into()
                .map_err(|_| error("invalid start header"))?,
        )
    {
        return Err(error("start-header checksum does not match"));
    }
    let size = number(&prefix[20..28]);
    if size == 0 || size > MAX_TOC {
        return Err(error(
            "stored or decoded metadata exceeds the 128 KiB limit",
        ));
    }
    let offset = number(&prefix[12..20])
        .checked_add(32)
        .ok_or_else(|| error("metadata offset overflow"))?;
    if offset.checked_add(size).is_none_or(|end| end > length) {
        return Err(error("metadata points outside the archive"));
    }
    source.seek(SeekFrom::Start(offset))?;
    let mut toc = vec![0; size as usize];
    for chunk in toc.chunks_mut(64 * 1024) {
        cancel.check()?;
        source.read_exact(chunk)?;
    }
    if crc32(&toc)
        != u32::from_le_bytes(
            prefix[28..32]
                .try_into()
                .map_err(|_| error("invalid metadata header"))?,
        )
    {
        return Err(error("metadata checksum does not match"));
    }
    match toc.first() {
        Some(0x01) => {}
        Some(0x17) => {
            // Reuse the library's StreamsInfo parser without asking it to parse
            // the decoded file table. The synthetic TOC describes exactly one
            // file whose payload is the original compressed metadata stream.
            let mut footer = vec![0x01, 0x04];
            footer.extend_from_slice(&toc[1..]);
            footer.extend_from_slice(&[0x05, 1, 0x11, 19, 0]);
            for value in "metadata\0".encode_utf16() {
                footer.extend_from_slice(&value.to_le_bytes());
            }
            footer.extend_from_slice(&[0, 0]);
            let probe = Probe::new(&mut *source, length, prefix, footer)?;
            let mut reader =
                ArchiveReader::new_with_memory_limit(probe, Password::empty(), 128 * 1024)
                    .map_err(|e| error(&e.to_string()))?;
            reader.set_thread_count(1);
            if reader.archive().files.len() != 1 || reader.archive().blocks.len() != 1 {
                return Err(error("encoded metadata must use a single bounded stream"));
            }
            if reader.archive().files[0].size > MAX_TOC {
                return Err(error(
                    "stored or decoded metadata exceeds the 128 KiB limit",
                ));
            }
            validate_blocks(reader.archive())?;
            let mut failure = None;
            let mut entries = 0;
            let mut decoded = Vec::new();
            reader
                .for_each_entries(|entry, payload| {
                    let result = (|| {
                        cancel.check()?;
                        if entry.is_directory() || entry.size() > MAX_TOC {
                            return Err(error("decoded metadata exceeds the 128 KiB limit"));
                        }
                        let mut buffer = [0; 8192];
                        loop {
                            cancel.check()?;
                            let count = payload.read(&mut buffer)?;
                            if count == 0 {
                                break;
                            }
                            if decoded.len() + count > MAX_TOC as usize {
                                return Err(error("decoded metadata exceeds the 128 KiB limit"));
                            }
                            decoded.extend_from_slice(&buffer[..count]);
                        }
                        if decoded.len() as u64 != entry.size() {
                            return Err(error("decoded metadata is truncated"));
                        }
                        entries += 1;
                        Ok(())
                    })();
                    if let Err(error) = result {
                        failure = Some(error);
                        return Err(sevenz_rust2::Error::Other(
                            "metadata policy rejected the archive".into(),
                        ));
                    }
                    Ok(true)
                })
                .map_err(|e| failure.take().unwrap_or_else(|| error(&e.to_string())))?;
            if entries != 1 {
                return Err(error("encoded metadata stream is missing"));
            }
            if decoded.first() != Some(&0x01) {
                return Err(error("decoded metadata must be a plain header"));
            }
            toc = decoded;
        }
        _ => return Err(error("metadata header type is unsupported")),
    }
    cancel.check()?;
    source.seek(SeekFrom::Start(0))?;
    Ok(Metadata {
        length,
        prefix,
        toc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::CancelToken;
    use std::io::{Cursor, Read, Seek, SeekFrom};

    fn fixture(compressed: bool) -> Vec<u8> {
        let mut writer = sevenz_rust2::ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
        for index in 0..if compressed { 100 } else { 1 } {
            let name = if compressed {
                format!("folder/repeated-name-for-header-{index}")
            } else {
                "empty".into()
            };
            writer
                .push_archive_entry(
                    sevenz_rust2::ArchiveEntry::new_directory(&name),
                    None::<Cursor<Vec<u8>>>,
                )
                .unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    #[test]
    fn real_plain_and_compressed_headers_pass_the_bounded_probe() {
        for compressed in [false, true] {
            let bytes = fixture(compressed);
            let offset = u64::from_le_bytes(bytes[12..20].try_into().unwrap()) as usize + 32;
            assert_eq!(bytes[offset], if compressed { 0x17 } else { 0x01 });
            let metadata =
                preflight(&mut Cursor::new(bytes.clone()), &CancelToken::default()).unwrap();
            let reader = metadata
                .reader(Cursor::new(bytes), &CancelToken::default())
                .unwrap();
            assert_eq!(
                reader.archive().files.len(),
                if compressed { 100 } else { 1 }
            );
        }
    }
    #[test]
    fn compressed_metadata_expansion_is_rejected_before_decoding_the_table() {
        let mut writer = sevenz_rust2::ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
        writer
            .push_archive_entry(
                sevenz_rust2::ArchiveEntry::new_directory(&"repeated-name-".repeat(10000)),
                None::<Cursor<Vec<u8>>>,
            )
            .unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert!(bytes.len() < 16 * 1024);
        let error = preflight(&mut Cursor::new(bytes), &CancelToken::default()).unwrap_err();
        assert!(error.to_string().contains("128 KiB"), "{error}");
    }
    #[test]
    fn stored_metadata_limit_is_checked_before_payload_reads() {
        struct HeaderOnly {
            header: Cursor<Vec<u8>>,
            length: u64,
            payload_reads: u32,
        }
        impl Read for HeaderOnly {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.header.position() >= 32 {
                    self.payload_reads += 1;
                    return Err(std::io::Error::other("unexpected payload read"));
                }
                self.header.read(out)
            }
        }
        impl Seek for HeaderOnly {
            fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
                match from {
                    SeekFrom::End(0) => Ok(self.length),
                    _ => self.header.seek(from),
                }
            }
        }
        let mut header = [0_u8; 32];
        header[..6].copy_from_slice(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c]);
        header[20..28].copy_from_slice(&(128 * 1024 + 1_u64).to_le_bytes());
        let crc = crc32(&header[12..]);
        header[8..12].copy_from_slice(&crc.to_le_bytes());
        let mut reader = HeaderOnly {
            header: Cursor::new(header.to_vec()),
            length: 32 + 128 * 1024 + 1,
            payload_reads: 0,
        };
        assert!(
            preflight(&mut reader, &CancelToken::default())
                .unwrap_err()
                .to_string()
                .contains("128 KiB")
        );
        assert_eq!(reader.payload_reads, 0);
    }
    #[test]
    fn virtual_probe_refuses_out_of_range_seeks_and_preserves_framing() {
        let bytes = fixture(false);
        let length = bytes.len() as u64;
        let mut source = Cursor::new(bytes.clone());
        let prefix: [u8; 32] = bytes[..32].try_into().unwrap();
        let footer = vec![1, 2, 3, 4, 5];
        let mut probe = Probe::new(&mut source, length, prefix, footer.clone()).unwrap();
        assert_eq!(probe.seek(SeekFrom::End(0)).unwrap(), length + 5);
        assert!(probe.seek(SeekFrom::End(-((length + 6) as i64))).is_err());
        assert!(probe.seek(SeekFrom::Start(length + 6)).is_err());
        probe.seek(SeekFrom::Start(length)).unwrap();
        let mut got = [0; 5];
        probe.read_exact(&mut got).unwrap();
        assert_eq!(got, footer.as_slice());
        assert_eq!(
            u64::from_le_bytes(probe.prefix[12..20].try_into().unwrap()),
            length - 32
        );
    }
    #[test]
    fn decoder_graph_budget_is_cumulative_and_rejects_unknown_or_huge_models() {
        use sevenz_rust2::EncoderMethod as M;
        assert!(validate_models([(M::ID_LZMA2, [28_u8].as_slice())].into_iter()).is_ok());
        assert!(
            validate_models(
                [
                    (M::ID_LZMA2, [28_u8].as_slice()),
                    (M::ID_LZMA2, [28_u8].as_slice())
                ]
                .into_iter()
            )
            .is_err()
        );
        assert!(validate_models([(M::ID_LZMA2, [40_u8].as_slice())].into_iter()).is_err());
        assert!(validate_models(std::iter::repeat_n((M::ID_COPY, &[][..]), 5)).is_err());
        assert!(validate_models([(M::ID_BCJ2, &[][..])].into_iter()).is_err());
    }
    #[test]
    fn malformed_file_count_is_rejected_from_a_tiny_snapshot() {
        // kHeader, kFilesInfo, maximal varint, kEnd, kEnd. The reader must
        // reject the count against the small metadata buffer before reserving.
        let mut toc = vec![0x01, 0x05, 0xff];
        toc.extend_from_slice(&u64::MAX.to_le_bytes());
        toc.extend_from_slice(&[0, 0]);
        let source = Cursor::new(vec![0; 32]);
        let mut prefix = [0; 32];
        prefix[..8].copy_from_slice(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c, 0, 4]);
        let mut probe = Probe::new(source, 32, prefix, toc).unwrap();
        let metadata = preflight(&mut probe, &CancelToken::default()).unwrap();
        let error = metadata
            .reader(probe, &CancelToken::default())
            .err()
            .unwrap();
        assert!(error.to_string().contains("num files exceeds"), "{error}");
    }
    #[test]
    fn final_parser_uses_the_validated_snapshot_after_source_changes() {
        let mut source = Cursor::new(fixture(true));
        let metadata = preflight(&mut source, &CancelToken::default()).unwrap();
        source.get_mut().fill(0xff);
        // Only directory records: no packed payload reads are needed. Neither
        // the changed envelope nor the original compressed header is reparsed.
        let mut reader = metadata.reader(source, &CancelToken::default()).unwrap();
        let mut entries = 0;
        reader
            .for_each_entries(|entry, _| {
                assert!(entry.is_directory());
                entries += 1;
                Ok(true)
            })
            .unwrap();
        assert_eq!(entries, 100);
    }
}
