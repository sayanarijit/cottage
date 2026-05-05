use crate::Project;
use age::ssh;
use age::x25519;
use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Recipient {
    X25519(x25519::Recipient),
    Ssh(ssh::Recipient),
}

impl From<Recipient> for Box<dyn age::Recipient> {
    fn from(val: Recipient) -> Self {
        match val {
            Recipient::X25519(r) => Box::new(r),
            Recipient::Ssh(r) => Box::new(r),
        }
    }
}

impl AsRef<dyn age::Recipient> for Recipient {
    fn as_ref(&self) -> &(dyn age::Recipient + 'static) {
        match self {
            Recipient::X25519(r) => r,
            Recipient::Ssh(r) => r,
        }
    }
}

pub type RecipientData = (Recipient, Vec<u8>);

pub fn parse_recipient(s: &str) -> Result<Recipient> {
    if s.starts_with("age1") {
        let recipient = age::x25519::Recipient::from_str(s)
            .map_err(|e| anyhow!("{}: could not parse age recipient", e))?;
        Ok(Recipient::X25519(recipient))
    } else if s.starts_with("ssh-") || s.starts_with("ecdsa-") {
        let recipient = age::ssh::Recipient::from_str(s)
            .map_err(|e| anyhow!("{:?}: could not parse SSH recipient", e))?;
        Ok(Recipient::Ssh(recipient))
    } else {
        Err(anyhow!("{}: unsupported recipient format", s))
    }
}

pub fn parse_recipients_file(path: PathBuf) -> Result<Box<dyn Iterator<Item = RecipientData>>> {
    log::debug!("{}: parsing recipients", path.display());
    let file = File::open(&path)
        .with_context(|| format!("{}: could not open recipients file", path.display()))?;
    let reader = BufReader::new(file);
    let iter = reader.lines().filter_map(move |line| match line {
        Ok(line) => {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                None
            } else {
                match parse_recipient(trimmed_line) {
                    Ok(recipient) => Some((
                        recipient,
                        line.split_whitespace()
                            .take(2)
                            .collect::<Vec<&str>>()
                            .join(" ")
                            .as_bytes()
                            .to_vec(),
                    )),
                    Err(e) => {
                        log::warn!("{}: could not parse recipient: {}", path.display(), e);
                        None
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("{}: could not read line: {}", path.display(), e);
            None
        }
    });

    Ok(Box::new(iter))
}

pub fn parse_recipients_dir(path: PathBuf) -> Box<dyn Iterator<Item = RecipientData>> {
    let iter = walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .flat_map(|e| match parse_recipients_file(e.path().to_path_buf()) {
            Ok(iter) => iter,
            Err(err) => {
                log::warn!(
                    "{}: could not parse recipients file: {}",
                    e.path().display(),
                    err
                );
                Box::new(std::iter::empty())
            }
        });
    Box::new(iter)
}

pub fn load_recipients(
    proj: &Project,
    recipients: Vec<String>,
    recipients_file: Vec<PathBuf>,
) -> Box<dyn Iterator<Item = RecipientData>> {
    log::debug!("loading recipients");

    if recipients.is_empty() && recipients_file.is_empty() {
        log::debug!("no recipients provided, looking for defaults");
        let default_recipients_path = proj.recipients_path();
        if default_recipients_path.is_dir()
            && default_recipients_path
                .read_dir()
                .map(|mut i| i.next().is_some())
                .unwrap_or(false)
        {
            log::debug!(
                "found default recipients directory at {}, parsing",
                default_recipients_path.display()
            );
            parse_recipients_dir(default_recipients_path.to_path_buf())
        } else if default_recipients_path.is_file() {
            log::debug!(
                "no default recipients directory found, looking for default recipients file at {}",
                default_recipients_path.display()
            );
            match parse_recipients_file(default_recipients_path.to_path_buf()) {
                Ok(iter) => iter,
                Err(err) => {
                    log::warn!(
                        "{}: could not parse default recipients file: {}",
                        default_recipients_path.display(),
                        err
                    );
                    Box::new(std::iter::empty())
                }
            }
        } else {
            log::debug!("no default recipients found");
            Box::new(std::iter::empty())
        }
    } else {
        log::debug!("parsing provided recipients");
        let iter = recipients
            .into_iter()
            .filter_map(|r| match parse_recipient(&r) {
                Ok(recipient) => Some((recipient, r.as_bytes().to_vec())),
                Err(e) => {
                    log::warn!("{}: could not parse recipient: {}", r, e);
                    None
                }
            })
            .chain(recipients_file.into_iter().flat_map(|f| match f.is_dir() {
                true => parse_recipients_dir(f.to_path_buf()),
                false => match parse_recipients_file(f.to_path_buf()) {
                    Ok(iter) => iter,
                    Err(err) => {
                        log::warn!("{}: could not parse recipients file: {}", f.display(), err);
                        Box::new(std::iter::empty())
                    }
                },
            }));
        Box::new(iter)
    }
}
