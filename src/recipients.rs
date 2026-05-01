use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::Project;

pub fn parse_recipient(s: &str) -> Result<Box<dyn age::Recipient + Send>> {
    if s.starts_with("age1") {
        let recipient = age::x25519::Recipient::from_str(s)
            .map_err(|e| anyhow!("{}: failed to parse age recipient", e))?;
        Ok(Box::new(recipient))
    } else if s.starts_with("ssh-") || s.starts_with("ecdsa-") {
        let recipient = age::ssh::Recipient::from_str(s)
            .map_err(|e| anyhow!("{:?}: failed to parse SSH recipient", e))?;
        Ok(Box::new(recipient))
    } else {
        Err(anyhow!("{}: unsupported recipient format", s))
    }
}

pub fn parse_recipients_file(
    path: &Path,
) -> Result<Vec<(Box<dyn age::Recipient + Send>, Vec<u8>)>> {
    let file = File::open(path)
        .with_context(|| format!("{}: failed to open recipients file", path.display()))?;
    let reader = BufReader::new(file);
    let mut recipients = Vec::new();

    use std::io::BufRead;
    for line in reader.lines() {
        let line = line?;
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
            continue;
        }
        recipients.push((parse_recipient(trimmed_line)?, line.into_bytes()));
    }

    Ok(recipients)
}

pub fn parse_recipients_dir(path: &Path) -> Result<Vec<(Box<dyn age::Recipient + Send>, Vec<u8>)>> {
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
    proj: &Project,
    recipients: &[String],
    recipients_file: &Vec<PathBuf>,
) -> Result<Vec<(Box<dyn age::Recipient + Send>, Vec<u8>)>> {
    let mut result = Vec::new();

    if recipients.is_empty() && recipients_file.is_empty() {
        let default_recipients = proj.recipients_path();
        if default_recipients.is_dir() && default_recipients.read_dir()?.next().is_some() {
            result.extend(parse_recipients_dir(&default_recipients)?);
        } else if default_recipients.is_file() {
            result.extend(parse_recipients_file(&default_recipients)?);
        }
    } else {
        for r in recipients {
            result.push((parse_recipient(r)?, r.as_bytes().to_vec()));
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
