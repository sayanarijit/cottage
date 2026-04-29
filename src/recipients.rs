use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;

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
