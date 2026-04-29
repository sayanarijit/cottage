use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
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

pub fn load_recipients(
    root: &Path,
    recipients: &[String],
    recipients_file: &Vec<PathBuf>,
) -> Result<Vec<Box<dyn age::Recipient + Send>>> {
    let mut result = Vec::new();

    if recipients.is_empty() && recipients_file.is_empty() {
        let default_recipients = root.join(".cottage/recipients");
        if default_recipients.is_dir() && default_recipients.read_dir()?.next().is_some() {
            result.extend(parse_recipients_dir(&default_recipients)?);
        } else if default_recipients.is_file() {
            result.extend(parse_recipients_file(&default_recipients)?);
        }
    } else {
        for r in recipients {
            result.push(parse_recipient(r)?);
        }
        for f in recipients_file {
            if f.is_dir() {
                result.extend(parse_recipients_dir(&f)?);
            } else if f.is_file() {
                result.extend(parse_recipients_file(&f)?);
            }
        }
    }
    Ok(result)
}
