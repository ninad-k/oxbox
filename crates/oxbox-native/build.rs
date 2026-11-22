//! Give app and test executables the same Windows Common Controls activation.
//!
//! The legacy workspace host can enable rfd's `common-controls-v6` feature for
//! the native host too. Its TaskDialogIndirect import needs the v6 assembly
//! before the executable starts, including when Cargo builds a test harness.
use std::{env, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=windows.manifest");
    if env::var("CARGO_CFG_TARGET_OS")?.as_str() == "windows"
        && env::var("CARGO_CFG_TARGET_ENV")?.as_str() == "msvc"
    {
        let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("windows.manifest");
        // These apply to the package's app and test targets, not its dependencies.
        // RT_MANIFEST is independent of the release wrapper's icon/version resource.
        println!("cargo::rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo::rustc-link-arg=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
    Ok(())
}
