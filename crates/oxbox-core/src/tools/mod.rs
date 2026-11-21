//! One module per tool. The module name is the tool id in `snake_case`
//! (tool id `verify-checksum` → module `checksum` is fine too; keep it obvious).
//!
//! Reference implementations to copy when adding a tool:
//! - [`text_transform`]: a pure, synchronous tool (no I/O).
//! - [`checksum`]: a file tool that streams data and runs on a blocking thread.

pub mod checksum;
pub mod text_transform;
