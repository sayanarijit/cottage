use crate::Project;
use age::ssh;
use age::x25519;
use anyhow::{Context, Result, anyhow};
use globset::Glob;
use globset::GlobMatcher;
use globset::GlobSet;
use globset::GlobSetBuilder;
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
    recipients_file: Vec<PathBuf>,
    globmatcher: Option<&'a GlobMatcher>,
) -> Box<dyn Iterator<Item = RecipientData> + 'a> {
    log::debug!("loading recipients");

    let mut paths = recipients_file.clone();
    if paths.is_empty() {
        log::debug!("no recipients provided, looking for defaults");
        paths.push(proj.recipients_path().into());
    }

    let iter = paths.into_iter().flat_map(move |path| {
        if path.is_dir()
            && path
                .read_dir()
                .map(|mut i| i.next().is_some())
                .unwrap_or(false)
        {
            parse_recipients_dir(path.to_path_buf(), proj.recipients_path(), globmatcher)
        } else if path.is_file() {
            if let Some(matcher) = globmatcher {
                if match_path(&path, proj.recipients_path(), Some(matcher)) {
                    parse_recipients_file(path.to_path_buf(), proj.recipients_path())
                        .unwrap_or_else(|e| {
                            log::warn!(
                                "{}: could not parse recipients file: {}",
                                path.display(),
                                e
                            );
                            Box::new(std::iter::empty())
                        })
                } else {
                    log::debug!(
                        "{}: skipping recipients file: path does not match glob rules",
                        path.display()
                    );
                    Box::new(std::iter::empty())
                }
            } else {
                parse_recipients_file(path.to_path_buf(), proj.recipients_path()).unwrap_or_else(
                    |e| {
                        log::warn!("{}: could not parse recipients file: {}", path.display(), e);
                        Box::new(std::iter::empty())
                    },
                )
            }
        } else {
            log::debug!("{}: path does not exist", path.display());
            Box::new(std::iter::empty())
        }
    });
    Box::new(iter)
}

fn build_matcher(globs: Option<&[Glob]>) -> Result<Option<GlobSet>> {
    let mut builder = GlobSetBuilder::new();
    if let Some(globs) = globs {
        for glob in globs {
            builder.add(glob.clone());
        }

        let res = builder.build()?;
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

pub fn filter_recipients_by_metadata<'a>(
    all_recipients: &'a [RecipientData],
    allow: Option<&'a [Glob]>,
    deny: Option<&'a [Glob]>,
) -> Result<Box<dyn Iterator<Item = RecipientData> + 'a>> {
    let allowmatcher = build_matcher(allow)?;
    let denymatcher = build_matcher(deny)?;

    let filtered = all_recipients.iter().filter_map(move |r| {
        match (r.path.as_ref(), &allowmatcher, &denymatcher) {
            (Some(ref path), _, Some(deny)) if deny.is_match(path) => {
                log::debug!(
                    "{}: skipping recipient: path matches deny rules",
                    path.display()
                );
                None
            }
            (Some(ref path), Some(allow), _) if !allow.is_match(path) => {
                log::debug!(
                    "{}: skipping recipient: path does not match allow rules",
                    path.display()
                );
                None
            }
            _ => Some(r.clone()),
        }
    });

    Ok(Box::new(filtered))
}

pub fn make_recipients_checksum_data(recipients: &[RecipientData]) -> Vec<u8> {
    recipients
        .iter()
        .flat_map(|r| r.raw.iter().chain(b"\n"))
        .copied()
        .collect::<Vec<_>>()
}
