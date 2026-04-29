use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::str::FromStr;

pub fn parse_identity_file(path: &Path) -> Result<Box<dyn age::Identity>> {
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read identity file: {:?}", path))?;

    if s.starts_with("AGE-SECRET-KEY-1") {
        let identity = age::x25519::Identity::from_str(&s)
            .map_err(|e| anyhow!("Failed to parse age identity: {}", e))?;
        return Ok(Box::new(identity));
    }

    let identity = age::ssh::Identity::from_buffer(s.as_bytes(), None)?;
    Ok(Box::new(identity))
}

pub fn parse_identities_dir(path: &Path) -> Vec<Box<dyn age::Identity>> {
    let mut identities = Vec::new();
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            match parse_identity_file(&entry.path()) {
                Ok(identity) => identities.push(identity),
                Err(e) => eprintln!("skipped: {}: {}", entry.path().display(), e),
            }
        }
    }
    identities
}
