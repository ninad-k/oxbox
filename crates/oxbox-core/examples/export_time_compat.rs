//! Export deterministic current time behavior to stdout before replacing its backend.
//! This example reads no clock, profile, system zone, device or network source.
#[path = "../tests/support/time_compat.rs"]
mod time_compat;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&time_compat::snapshot()?)?
    );
    Ok(())
}
