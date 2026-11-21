# ADR 0003: Require a memory-limited 7z decoder

- Status: Accepted
- Date: 2026-11-21

## Context

An output-byte guard runs after a decoder has parsed archive metadata. The initially considered
`sevenz-rust2` 0.19 API can allocate archive-controlled metadata and dictionaries before our extraction callback.
Limiting only compressed header size does not bound decoded header size or decoder memory.

## Decision

Use `sevenz-rust2` 0.25 with `ArchiveReader::new_with_memory_limit` (its argument is in KiB), one decoding
thread, entry/path validation, and the shared extraction-output budget. Keep the resolved version in
`Cargo.lock`. Raise workspace MSRV from Rust 1.90 to 1.93, which this dependency requires.

Before the reader parses the file table, limit both stored and decoded 7z metadata to **128 KiB**. Verify the
start-header and stored-metadata checksums and checked offset ranges. For an encoded header, present its
StreamsInfo through a small synthetic plain header describing one metadata stream. Parse that graph with
the dependency, reject a declared decoded size above 128 KiB before decoding, and read the result in bounded
chunks with cancellation and CRC validation. Only a single encoded-metadata block is accepted. The final
reader receives the frozen plain metadata through a seekable virtual archive; packed data retain their
original offsets. It never reparses an unchecked or subsequently changed encoded header.

Before decoding metadata or file data, require at most four coders per block and a cumulative **128 MiB
decoder-model estimate**. Support Copy, LZMA/LZMA2, PPMd, BZip2, Delta and the standard single-input BCJ
filters; reject other methods and BCJ2. The estimate includes dictionary/model storage and conservative
overhead. The dependency's independent 128 MiB per-decoder limit remains in force. Entry count is limited
to 100,000 after parsing the small bounded table; the dependency rejects counts larger than the available
metadata bytes before reserving their tables.

## Consequences

Valid archives with large metadata, large dictionaries, unusual codec graphs or other compression methods
will fail with a resource-policy error. This conservative compatibility limit is deliberate. Future decoder
upgrades must retain the limits and hostile-archive tests. This change is within the archive task's approved
dependency set and adds no runtime network behavior.

Neither the decoder-model estimate nor the library setting is a process-wide resident-memory ceiling.
Metadata tables, stream maps, buffering, runtime overhead and the rest of the app use additional memory.
The 128 KiB metadata policy bounds the byte-based counts used to reserve those tables; it does not assert
that their in-memory representation occupies only 128 KiB. Continue the recorded human file-tool security
review and consider process isolation for stronger protection against decoder defects.

Primary API reference: [ArchiveReader](https://docs.rs/sevenz-rust2/0.25.0/sevenz_rust2/struct.ArchiveReader.html).
