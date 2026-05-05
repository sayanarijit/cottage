use crate::Project;
use age::ssh;
use age::x25519;
use anyhow::{Context, Result, anyhow};
use globset::GlobMatcher;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
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

#[derive(Debug, Clone)]
pub struct RecipientData {
    pub recipient: Recipient,
    pub raw: Vec<u8>,
    pub path: Option<PathBuf>,
}

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

pub fn parse_recipients_file<'a>(
    path: PathBuf,
    root: &'a Path,
) -> Result<Box<dyn Iterator<Item = RecipientData> + 'a>> {
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
                    Ok(recipient) => Some(RecipientData {
                        recipient,
                        path: Some(
                            pathdiff::diff_paths(&path, root).unwrap_or_else(|| path.clone()),
                        ),
                        raw: line
                            .split_whitespace()
                            .take(2)
                            .collect::<Vec<&str>>()
                            .join(" ")
                            .as_bytes()
                            .to_vec(),
                    }),
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

fn match_path(path: &Path, root: &Path, globmatcher: Option<&GlobMatcher>) -> bool {
    if let Some(matcher) = globmatcher {
        let relative_path = pathdiff::diff_paths(path, root).unwrap_or_else(|| path.to_path_buf());
        log::debug!("{}: checking against glob", relative_path.display());
        matcher.is_match(&relative_path)
    } else {
        log::debug!("{}: no glob provided, including by default", path.display());
        true
    }
}

pub fn parse_recipients_dir<'a>(
    path: PathBuf,
    root: &'a Path,
    globmatcher: Option<&'a GlobMatcher>,
) -> Box<dyn Iterator<Item = RecipientData> + 'a> {
    let iter = walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(move |e| {
            let path = e.path();
            path.is_file() && match_path(path, root, globmatcher)
        })
        .flat_map(
            |e| match parse_recipients_file(e.path().to_path_buf(), root) {
                Ok(iter) => iter,
                Err(err) => {
                    log::warn!(
                        "{}: could not parse recipients file: {}",
                        e.path().display(),
                        err
                    );
                    Box::new(std::iter::empty())
                }
            },
        );
    Box::new(iter)
}

pub fn load_recipients<'a>(
    proj: &'a Project,
    recipients: Vec<String>,
    recipients_file: Vec<PathBuf>,
    globmatcher: Option<&'a GlobMatcher>,
) -> Box<dyn Iterator<Item = RecipientData> + 'a> {
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
            parse_recipients_dir(
                default_recipients_path.to_path_buf(),
                proj.recipients_path(),
                globmatcher,
            )
        } else if default_recipients_path.is_file() && globmatcher.is_none() {
            log::debug!(
                "no default recipients directory found, looking for default recipients file at {}",
                default_recipients_path.display()
            );
            match parse_recipients_file(
                default_recipients_path.to_path_buf(),
                proj.recipients_path(),
            ) {
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
                Ok(recipient) => Some(RecipientData {
                    recipient,
                    raw: r.as_bytes().to_vec(),
                    path: None,
                }),
                Err(e) => {
                    log::warn!("{}: could not parse recipient: {}", r, e);
                    None
                }
            })
            .chain(
                recipients_file
                    .into_iter()
                    .flat_map(move |f| match f.is_dir() {
                        true => parse_recipients_dir(f, proj.recipients_path(), globmatcher),
                        false => {
                            match parse_recipients_file(f.to_path_buf(), proj.recipients_path()) {
                                Ok(iter) => iter,
                                Err(err) => {
                                    log::warn!(
                                        "{}: could not parse recipients file: {}",
                                        f.display(),
                                        err
                                    );
                                    Box::new(std::iter::empty())
                                }
                            }
                        }
                    }),
            );
        Box::new(iter)
    }
}
