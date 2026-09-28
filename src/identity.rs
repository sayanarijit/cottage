use crate::Project;
use age::ssh;
use age::x25519;
use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
pub enum Identity {
    X25519(x25519::Identity),
    Ssh(ssh::Identity),
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Identity::X25519(_) => write!(f, "X25519(*****)"),
            Identity::Ssh(_) => write!(f, "SSH(*****)"),
        }
    }
}

impl From<Identity> for Box<dyn age::Identity> {
    fn from(val: Identity) -> Self {
        match val {
            Identity::X25519(id) => Box::new(id),
            Identity::Ssh(id) => Box::new(id),
        }
    }
}

pub fn parse_identity_str(s: &str) -> Result<Box<dyn Iterator<Item = Identity>>> {
    let s = s
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    if s.starts_with("AGE-SECRET-KEY-1") {
        let mut ids = vec![];
        for line in s.lines() {
            let identity = age::x25519::Identity::from_str(line)
                .map_err(|e| anyhow!("{}: could not parse age identity", e))?;
            log::debug!("parsed age identity");
            ids.push(Identity::X25519(identity));
        }
        return Ok(Box::new(ids.into_iter()));
    }

    let identity = age::ssh::Identity::from_buffer(s.as_bytes(), None)
        .map_err(|e| anyhow!("{}: could not parse ssh identity", e))?;
    log::debug!("parsed ssh identity");
    Ok(Box::new(std::iter::once(Identity::Ssh(identity))))
}

pub fn parse_identity_file(path: &Path) -> Result<Box<dyn Iterator<Item = Identity>>> {
    log::debug!("{}: parsing identity", path.display());
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("{}: could not read identity file", path.display()))?;
    parse_identity_str(&s)
}

pub fn parse_identities_dir(path: &Path) -> Box<dyn Iterator<Item = Identity>> {
    log::debug!("{}: parsing identities in directory", path.display());
    let iter = ignore::WalkBuilder::new(path)
        .sort_by_file_name(|a, b| a.cmp(b))
        .standard_filters(false)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && !e.file_name().to_string_lossy().ends_with(".pub"))
        .map(|entry| entry.path().to_path_buf())
        .filter_map(move |p| match parse_identity_file(&p) {
            Ok(identities) => Some(identities),
            Err(e) => {
                log::warn!("skipped: {}: {}", p.display(), e);
                None
            }
        })
        .flatten();
    Box::new(iter)
}

#[allow(dead_code)]
pub fn parse_identities_path(path: &Path) -> Option<Box<dyn Iterator<Item = Identity>>> {
    if path.is_dir()
        && path
            .read_dir()
            .map(|mut i| i.next().is_some())
            .unwrap_or(false)
    {
        Some(parse_identities_dir(path))
    } else if path.is_file() {
        match parse_identity_file(path) {
            Ok(identities) => Some(identities),
            Err(e) => {
                log::warn!("{}: could not parse identity file: {}", path.display(), e);
                None
            }
        }
    } else {
        log::debug!("{}: path does not exist", path.display());
        None
    }
}

pub fn load_identities(
    proj: &Project,
    identities: Vec<String>,
) -> Box<dyn Iterator<Item = Identity>> {
    log::debug!("loading identities");
    if identities.is_empty() {
        log::debug!("no identities provided, looking for defaults");
        let project_identity_path = proj.project_identity_dir();
        let global_identity_path = proj.global_identity_path();
        let ssh_dir = proj.ssh_dir();

        if project_identity_path.is_dir() {
            log::debug!(
                "found default identities in {}",
                project_identity_path.display()
            );
            parse_identities_dir(project_identity_path)
        } else if global_identity_path.is_dir() {
            log::debug!(
                "found default identities in {}",
                global_identity_path.display()
            );
            parse_identities_dir(global_identity_path)
        } else if ssh_dir.is_dir() {
            log::debug!("no default identities found, looking in ~/.ssh");
            parse_identities_dir(ssh_dir)
        } else {
            log::debug!("no default identities found");
            Box::new(std::iter::empty())
        }
    } else {
        log::debug!("{} identities provided, parsing", identities.len());
        let iter = identities
            .into_iter()
            .filter_map(|item| {
                let path = Path::new(&item);
                if path.is_file() {
                    match parse_identity_file(path) {
                        Ok(identities) => Some(identities),
                        Err(e) => {
                            log::warn!("skipped: {}: {}", path.display(), e);
                            None
                        }
                    }
                } else if path.is_dir() {
                    Some(parse_identities_dir(path))
                } else {
                    match parse_identity_str(&item) {
                        Ok(identities) => Some(identities),
                        Err(e) => {
                            log::warn!("skipped: could not parse identity: {}", e);
                            None
                        }
                    }
                }
            })
            .flatten();
        Box::new(iter)
    }
}
