// Original Oxbox generated-fixture producer. Compile only with the exact
// age_old = { package = "age", version = "=0.11.5" } dependency documented here.
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

const PUBLIC_FIXTURE_PASSPHRASE: &str = "Oxbox public age 0.11.5 fixture password";
const PLAINTEXT: &[u8] = include_bytes!("plaintext.bin");

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: generate-age-0115-fixture <new generated-output.age>".into());
    }
    let output = PathBuf::from(&args[1]);
    let mut recipient = age_old::scrypt::Recipient::new(age_old::secrecy::SecretString::from(
        PUBLIC_FIXTURE_PASSPHRASE.to_string(),
    ));
    recipient.set_work_factor(10);
    let encryptor = age_old::Encryptor::with_recipients(std::iter::once(
        &recipient as &dyn age_old::Recipient,
    ))?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    let mut encrypted = encryptor.wrap_output(file)?;
    encrypted.write_all(PLAINTEXT)?;
    encrypted.finish()?.sync_all()?;

    // Self-check with the same historical reader before recording the artifact.
    let decryptor = age_old::Decryptor::new_buffered(BufReader::new(File::open(&output)?))?;
    assert!(decryptor.is_scrypt());
    let mut identity = age_old::scrypt::Identity::new(age_old::secrecy::SecretString::from(
        PUBLIC_FIXTURE_PASSPHRASE.to_string(),
    ));
    identity.set_max_work_factor(20);
    let reader = decryptor.decrypt(std::iter::once(&identity as &dyn age_old::Identity))?;
    let mut restored = Vec::new();
    reader
        .take(PLAINTEXT.len() as u64 + 1)
        .read_to_end(&mut restored)?;
    assert!(
        restored == PLAINTEXT,
        "historical fixture self-check failed"
    );
    println!(
        "Generated age 0.11.5 binary fixture; factor 10; {} plaintext bytes; historical self-check passed.",
        PLAINTEXT.len()
    );
    Ok(())
}
