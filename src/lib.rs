use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub enum EncryptionMode<'a> {
    Passphrase(String),
    Recipients(&'a [Box<dyn age::Recipient + Send>]),
}

pub enum DecryptionMode<'a> {
    Passphrase(String),
    Identities(&'a [Box<dyn age::Identity>]),
}

pub struct EncryptOptions<'a> {
    pub mode: EncryptionMode<'a>,
    pub armor: bool,
}

pub struct DecryptOptions<'a> {
    pub mode: DecryptionMode<'a>,
}

pub fn encrypt_file<'a>(
    input_path: &'a Path,
    options: &EncryptOptions,
) -> Result<(&'a Path, PathBuf)> {
    let input_file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {:?}", input_path))?;

    let output_path = input_path
        .with_added_extension("cott")
        .with_added_extension("age");

    let mut reader = BufReader::new(input_file);

    let output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {:?}", &output_path))?;
    let writer = BufWriter::new(output_file);

    let encryptor = match &options.mode {
        EncryptionMode::Passphrase(pass) => {
            age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(pass.as_str()))
        }
        EncryptionMode::Recipients(recipients) => age::Encryptor::with_recipients(
            recipients.iter().map(|r| r.as_ref() as &dyn age::Recipient),
        )
        .map_err(|_| anyhow!("At least one recipient must be provided"))?,
    };

    let mut writer: Box<dyn Write> = if options.armor {
        Box::new(age::armor::ArmoredWriter::wrap_output(
            writer,
            age::armor::Format::AsciiArmor,
        )?)
    } else {
        Box::new(writer)
    };

    let mut encrypting_writer = encryptor.wrap_output(&mut writer)?;
    std::io::copy(&mut reader, &mut encrypting_writer)?;
    encrypting_writer.finish()?;
    writer.flush()?;

    Ok((input_path, output_path))
}

pub fn encrypt_dir<'a>(
    path: &'a Path,
    options: &EncryptOptions,
) -> impl Iterator<Item = Result<(PathBuf, PathBuf)>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().to_string_lossy().ends_with(".cott.age"))
        .filter_map(|e| {
            e.path().file_stem().and_then(|s| {
                PathBuf::from(s)
                    .file_stem()
                    .map(|s| e.path().with_file_name(s))
            })
        })
        .map(|path| {
            encrypt_file(&path, options).map(|(input, output)| (input.to_path_buf(), output))
        })
}

pub fn parse_recipient(s: &str) -> Result<Box<dyn age::Recipient + Send>> {
    if s.starts_with("age1") {
        let recipient = age::x25519::Recipient::from_str(s)
            .map_err(|e| anyhow!("Failed to parse age recipient: {}", e))?;
        Ok(Box::new(recipient))
    } else if s.starts_with("ssh-") || s.starts_with("ecdsa-") {
        let recipient = age::ssh::Recipient::from_str(s)
            .map_err(|e| anyhow!("Failed to parse SSH recipient: {:?}", e))?;
        Ok(Box::new(recipient))
    } else {
        Err(anyhow!("Unsupported recipient format: {}", s))
    }
}

pub fn parse_identity_file(path: &Path) -> Result<Box<dyn age::Identity>> {
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read identity file: {:?}", path))?;

    if s.starts_with("AGE-SECRET-KEY-1") {
        let identity = age::x25519::Identity::from_str(&s)
            .map_err(|e| anyhow!("Failed to parse age identity: {}", e))?;
        return Ok(Box::new(identity));
    }

    if let Ok(identity) = age::ssh::Identity::from_buffer(s.as_bytes(), None) {
        match identity {
            age::ssh::Identity::Unsupported(k) => {
                // Skip unsupported keys, but log a warning
                eprintln!("skipped: {:?}: {:?}", path, k);
            }
            _ => return Ok(Box::new(identity)),
        }
    }

    Err(anyhow!("Unsupported identity format"))
}

pub fn parse_identities_dir(path: &Path) -> Result<Vec<Box<dyn age::Identity>>> {
    let mut identities = Vec::new();
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            identities.push(parse_identity_file(&entry.path())?);
        }
    }
    Ok(identities)
}

pub fn parse_recipients_file(path: &Path) -> Result<Vec<Box<dyn age::Recipient + Send>>> {
    let file =
        File::open(path).with_context(|| format!("Failed to open recipients file: {:?}", path))?;
    let reader = BufReader::new(file);
    let mut recipients = Vec::new();

    use std::io::BufRead;
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        recipients.push(parse_recipient(line)?);
    }

    Ok(recipients)
}

pub fn parse_recipients_dir(path: &Path) -> Result<Vec<Box<dyn age::Recipient + Send>>> {
    let mut recipients = Vec::new();
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            recipients.extend(parse_recipients_file(&entry.path())?);
        }
    }
    Ok(recipients)
}
