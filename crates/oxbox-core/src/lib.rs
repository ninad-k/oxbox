//! `oxbox-core` holds the logic for every Oxbox tool.
//!
//! Rules for this crate (see `AGENTS.md`):
//! - No Tauri or UI dependencies. Everything here must be callable from a test or a CLI.
//! - One module per tool under [`tools`]. OS-specific code goes in [`platform`].
//! - Return [`CoreError`]; never panic on user input.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod fs_safe;
pub mod jobs;
pub mod launcher;
pub mod platform;
pub mod preferences;
pub mod startup;
pub mod tools;

pub use error::{CoreError, CoreResult};
