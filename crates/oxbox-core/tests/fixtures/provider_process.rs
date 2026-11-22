//! Dependency-free, self-terminating provider fixture. Never opens devices or the network.
use std::{
    io::{self, Write},
    process::{Command, Stdio},
    time::Duration,
};

pub enum Mode {
    Version,
    WrapperExit,
    WrapperWait,
    Descendant,
}

pub fn run(mode: Mode, child_arguments: &[&str]) -> io::Result<()> {
    match mode {
        Mode::Version => {
            std::thread::sleep(Duration::from_millis(500));
            println!("Oxbox local provider fixture 1.0");
        }
        Mode::Descendant => {
            println!("FIXTURE_DESCENDANT_READY");
            io::stdout().flush()?;
            std::thread::sleep(Duration::from_secs(4));
        }
        Mode::WrapperExit | Mode::WrapperWait => {
            let mut command = Command::new(std::env::current_exe()?);
            command
                .args(child_arguments)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            // Deliberately inherit the provider pipes. This bounded descendant
            // exits by itself even against the pre-fix implementation.
            let _descendant = command.spawn()?;
            std::thread::sleep(Duration::from_millis(150));
            println!("FIXTURE_WRAPPER_READY");
            io::stdout().flush()?;
            if matches!(mode, Mode::WrapperWait) {
                std::thread::sleep(Duration::from_secs(4));
            }
        }
    }
    Ok(())
}

#[cfg(not(test))]
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mode = match args.first().map(String::as_str) {
        Some("--fixture-descendant") => Mode::Descendant,
        Some("--fixture-wrapper-exit") => Mode::WrapperExit,
        Some("--fixture-wrapper-wait") => Mode::WrapperWait,
        _ => Mode::Version,
    };
    run(mode, &["--fixture-descendant"])
}
