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

impl From<Identity> for Box<dyn age::Identity> {
    fn from(val: Identity) -> Self {
        match val {
            Identity::X25519(id) => Box::new(id),
            Identity::Ssh(id) => Box::new(id),
        }
    }
}

impl AsRef<dyn age::Identity> for Identity {
    fn as_ref(&self) -> &(dyn age::Identity + 'static) {
        match self {
            Identity::X25519(id) => id,
            Identity::Ssh(id) => id,
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

pub fn load_identities(
    proj: &Project,
    identities: Vec<PathBuf>,
) -> Box<dyn Iterator<Item = Identity>> {
    log::debug!("loading identities");
    if identities.is_empty() {
        log::debug!("no identities provided, looking for defaults");
        let default_identities_path = proj.identity_path();
        if default_identities_path.is_dir()
            && default_identities_path
                .read_dir()
                .map(|mut i| i.next().is_some())
                .unwrap_or(false)
        {
            log::debug!(
                "found default identities directory at {}, parsing",
                default_identities_path.display()
            );
            parse_identities_dir(default_identities_path)
        } else if default_identities_path.is_file() {
            log::debug!(
                "no default identities directory found, looking for default identity file at {}",
                default_identities_path.display()
            );
            match parse_identity_file(default_identities_path) {
                Ok(identity) => Box::new(std::iter::once(identity)),
                Err(e) => {
                    log::warn!(
                        "{}: could not parse default identity: {}",
                        default_identities_path.display(),
                        e
                    );
                    Box::new(std::iter::empty())
                }
            }
        } else {
            log::debug!("no default identities found, looking in ~/.ssh");
            if let Some(sshdir) = dirs::home_dir().map(|h| h.join(".ssh")) {
                if sshdir.is_dir()
                    && sshdir
                        .read_dir()
                        .map(|mut i| i.next().is_some())
                        .unwrap_or(false)
                {
                    Box::new(parse_identities_dir(&sshdir))
                } else {
                    log::debug!("no identities found in ~/.ssh");
                    Box::new(std::iter::empty())
                }
            } else {
                log::debug!("no home directory found, cannot look for identities in ~/.ssh");
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
