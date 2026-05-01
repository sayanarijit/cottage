use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::Project;

pub fn parse_identity_file(path: &Path) -> Result<Box<dyn age::Identity>> {
    log::debug!("{}: parsing identity", path.display());
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("{}: failed to read identity file", path.display()))?;

    if s.starts_with("AGE-SECRET-KEY-1") {
        let identity = age::x25519::Identity::from_str(&s)
            .map_err(|e| anyhow!("{}: failed to parse age identity", e))?;
        log::debug!("{}: parsed age identity", path.display());
        return Ok(Box::new(identity));
    }

    let identity = age::ssh::Identity::from_buffer(s.as_bytes(), None)?;
    log::debug!("{}: parsed ssh identity", path.display());
    Ok(Box::new(identity))
}

pub fn parse_identities_dir(path: &Path) -> Vec<Box<dyn age::Identity>> {
    let mut identities = Vec::new();
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            match parse_identity_file(entry.path()) {
                Ok(identity) => identities.push(identity),
                Err(e) => log::warn!("skipped: {}: {}", entry.path().display(), e),
            }
        }
    }
    identities
}

pub fn load_identities(
    proj: &Project,
    identities: &[PathBuf],
) -> Result<Vec<Box<dyn age::Identity>>> {
    let mut result = Vec::new();

    if identities.is_empty() {
        let default_identities = proj.identity_path();
        if default_identities.is_dir() && default_identities.read_dir()?.next().is_some() {
            result.extend(parse_identities_dir(default_identities));
        } else if default_identities.is_file() {
            result.push(parse_identity_file(default_identities)?);
        } else {
            let sshdir = dirs::home_dir()
                .ok_or_else(|| anyhow!("failed to get home directory"))?
                .join(".ssh");
            if sshdir.is_dir() {
                result.extend(parse_identities_dir(&sshdir));
            }
        }
    } else {
        for i in identities {
            match parse_identity_file(i) {
                Ok(identity) => result.push(identity),
                Err(e) => {
                    log::warn!("skipped: {}: {}", i.display(), e);
                }
            }
        }
    }
    log::debug!("loaded {} identities", result.len());
    Ok(result)
}
