use crate::Project;
use age::ssh;
use age::x25519;
use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
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

pub fn parse_identity_file(path: &Path) -> Result<Identity> {
    log::debug!("{}: parsing identity", path.display());
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("{}: could not read identity file", path.display()))?;

    if s.starts_with("AGE-SECRET-KEY-1") {
        let identity = age::x25519::Identity::from_str(&s)
            .map_err(|e| anyhow!("{}: could not parse age identity", e))?;
        log::debug!("{}: parsed age identity", path.display());
        return Ok(Identity::X25519(identity));
    }

    let identity = age::ssh::Identity::from_buffer(s.as_bytes(), None)?;
    log::debug!("{}: parsed ssh identity", path.display());
    Ok(Identity::Ssh(identity))
}

pub fn parse_identities_dir(path: &Path) -> Box<dyn Iterator<Item = Identity>> {
    log::debug!("{}: parsing identities in directory", path.display());
    let iter = walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && !e.file_name().to_string_lossy().ends_with(".pub"))
        .map(|entry| entry.path().to_path_buf())
        .filter_map(move |p| match parse_identity_file(&p) {
            Ok(identity) => Some(identity),
            Err(e) => {
                log::warn!("skipped: {}: {}", p.display(), e);
                None
            }
        });
    Box::new(iter)
}

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
            Ok(identity) => Some(Box::new(std::iter::once(identity))),
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
    identities: Vec<PathBuf>,
) -> Box<dyn Iterator<Item = Identity>> {
    log::debug!("loading identities");
    if identities.is_empty() {
        log::debug!("no identities provided, looking for defaults");
        let local_identity_path = proj.identity_path();
        let global_identity_path = proj.global_identity_path();
        if let Some(ids) = parse_identities_path(local_identity_path) {
            log::debug!(
                "found default identities in {}",
                local_identity_path.display()
            );
            ids
        } else if let Some(ids) = parse_identities_path(global_identity_path) {
            log::debug!(
                "found default identities in {}",
                global_identity_path.display()
            );
            ids
        } else {
            log::debug!("no default identities found, looking in ~/.ssh");
            let sshdir = proj.ssh_dir();

            if sshdir.is_dir()
                && sshdir
                    .read_dir()
                    .map(|mut i| i.next().is_some())
                    .unwrap_or(false)
            {
                Box::new(parse_identities_dir(sshdir))
            } else {
                log::debug!("no identities found in ~/.ssh");
                Box::new(std::iter::empty())
            }
        }
    } else {
        log::debug!("{} identities provided, parsing", identities.len());
        let iter = identities
            .into_iter()
            .filter_map(|p| match parse_identity_file(&p) {
                Ok(identity) => Some(identity),
                Err(e) => {
                    log::warn!("skipped: {}: {}", p.display(), e);
                    None
                }
            });
        Box::new(iter)
    }
}
