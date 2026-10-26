use crate::secure_remove_file;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// `[path.prefix]` section of cottage.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PrefixConfig {
    pub encrypted: Option<PathBuf>,
    pub metadata: Option<PathBuf>,
}

/// `[path]` section of cottage.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PathConfig {
    pub prefix: Option<PrefixConfig>,
}

/// Resolved (absolute) prefix directories of a project.
#[derive(Debug, Clone, Default)]
pub struct PathPrefix {
    root: PathBuf,
    encrypted: Option<PathBuf>,
    metadata: Option<PathBuf>,
}

pub fn is_encrypted_path(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".cott.age")
}

pub fn is_metadata_path(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".cott.toml")
}

impl PathPrefix {
    pub fn new(root: &Path, config: Option<&PathConfig>) -> Self {
        let prefix = config.and_then(|c| c.prefix.as_ref());
        let resolve = |p: &PathBuf| {
            let joined = root.join(p);
            joined
                .components()
                .filter(|c| !matches!(c, std::path::Component::CurDir))
                .collect::<PathBuf>()
        };
        Self {
            root: root.to_path_buf(),
            encrypted: prefix.and_then(|p| p.encrypted.as_ref()).map(resolve),
            metadata: prefix.and_then(|p| p.metadata.as_ref()).map(resolve),
        }
    }

    pub fn encrypted_dir(&self) -> Option<&Path> {
        self.encrypted.as_deref()
    }

    pub fn metadata_dir(&self) -> Option<&Path> {
        self.metadata.as_deref()
    }

    /// Creates the declared prefix directories.
    pub fn create_dirs(&self) -> Result<()> {
        for dir in self.encrypted.iter().chain(self.metadata.iter()) {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("{}: could not create directory", dir.display()))?;
        }
        Ok(())
    }

    /// Maps `path` (relative to the project root) under `dir`, or returns None if
    /// no prefix is declared or `path` is outside the project.
    fn under(&self, dir: Option<&Path>, path: &Path) -> Option<PathBuf> {
        let dir = dir?;
        let abs = std::path::absolute(path).ok()?;
        Some(dir.join(abs.strip_prefix(&self.root).ok()?))
    }

    /// The directory under the encrypted prefix that mirrors the directory `path`.
    pub fn encrypted_dir_of(&self, path: &Path) -> Option<PathBuf> {
        self.under(self.encrypted_dir(), path)
    }

    pub fn to_encrypted_path(&self, path: &Path) -> PathBuf {
        self.under(self.encrypted_dir(), path)
            .unwrap_or_else(|| path.to_path_buf())
            .with_added_extension("cott")
            .with_added_extension("age")
    }

    pub fn to_metadata_path(&self, path: &Path) -> PathBuf {
        self.under(self.metadata_dir(), path)
            .unwrap_or_else(|| path.to_path_buf())
            .with_added_extension("cott")
            .with_added_extension("toml")
    }

    /// The decrypted path of an encrypted or metadata path, or None if it is neither.
    pub fn to_decrypted_path(&self, path: &Path) -> Option<PathBuf> {
        if !is_encrypted_path(path) && !is_metadata_path(path) {
            return None;
        }
        let path = self.unprefixed(path).unwrap_or_else(|| path.to_path_buf());
        path.file_stem()
            .and_then(|s| PathBuf::from(s).file_stem().map(|s| path.with_file_name(s)))
    }

    /// Walks `path` for encrypted secrets, including those mirrored under the encrypted prefix.
    pub fn iter_encrypted(&self, path: &Path) -> impl Iterator<Item = ignore::DirEntry> + use<> {
        fn walk(path: &Path) -> ignore::Walk {
            let mut builder = ignore::WalkBuilder::new(path);
            builder
                .sort_by_file_name(|a, b| a.cmp(b))
                .standard_filters(true)
                .hidden(false);
            builder.build()
        }

        let prefixed = self
            .encrypted_dir_of(path)
            .filter(|p| path.is_dir() && p.is_dir());
        let walks = std::iter::once(path.to_path_buf()).chain(prefixed);

        let mut seen = std::collections::HashSet::new();
        walks
            .flat_map(|p| walk(&p))
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| is_encrypted_path(e.path()))
            .filter(move |e| seen.insert(e.path().to_path_buf()))
    }

    /// Strips the prefix directory from a prefixed encrypted/metadata path, if it has one.
    pub fn unprefixed(&self, path: &Path) -> Option<PathBuf> {
        let dir = if is_encrypted_path(path) {
            self.encrypted_dir()
        } else if is_metadata_path(path) {
            self.metadata_dir()
        } else {
            None
        }?;
        let abs = std::path::absolute(path).ok()?;
        Some(self.root.join(abs.strip_prefix(dir).ok()?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Sync,
    Revert,
}

fn collect_files(dir: &Path, standard_filters: bool, out: &mut Vec<PathBuf>) {
    let walk = ignore::WalkBuilder::new(dir)
        .standard_filters(standard_filters)
        .hidden(false)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build();
    out.extend(
        walk.filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.is_file() && (is_encrypted_path(p) || is_metadata_path(p))),
    );
}

fn move_file(from: &Path, to: &Path, dry_run: bool) -> Result<()> {
    if from == to {
        return Ok(());
    }
    if to.exists() {
        anyhow::bail!(
            "{}: cannot move {}: destination already exists",
            to.display(),
            from.display()
        );
    }
    if dry_run {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("{}: could not create directory", parent.display()))?;
    }
    if std::fs::rename(from, to).is_err() {
        std::fs::copy(from, to)
            .with_context(|| format!("{}: could not copy to {}", from.display(), to.display()))?;
        secure_remove_file(from)?;
    }
    Ok(())
}

/// Removes now-empty directories left behind under `dir`, bottom up (best effort).
fn prune_empty_dirs(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            prune_empty_dirs(&entry.path());
            let _ = std::fs::remove_dir(entry.path());
        }
    }
}

fn relocate(
    paths: &[PathBuf],
    prefix: &PathPrefix,
    direction: Direction,
    dry_run: bool,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut moves = vec![];
    for path in paths {
        let Some(decrypted) = prefix.to_decrypted_path(path) else {
            continue;
        };
        let target = match (direction, is_encrypted_path(path)) {
            (Direction::Sync, true) => prefix.to_encrypted_path(&decrypted),
            (Direction::Sync, false) => prefix.to_metadata_path(&decrypted),
            (Direction::Revert, true) => decrypted.with_added_extension("cott.age"),
            (Direction::Revert, false) => decrypted.with_added_extension("cott.toml"),
        };
        if &target != path {
            move_file(path, &target, dry_run)?;
            moves.push((path.clone(), target));
        }
    }
    Ok(moves)
}

/// Moves stray encrypted secrets and metadata under `path` to their prefixed location.
pub fn prefix_sync(
    prefix: &PathPrefix,
    path: &Path,
    dry_run: bool,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    if !dry_run {
        prefix.create_dirs()?;
    }
    let mut files = vec![];
    if path.is_file() {
        files.push(path.to_path_buf());
    } else {
        collect_files(path, true, &mut files);
    }
    // Files already in a prefix directory map to themselves; only strays move.
    files.retain(|f| prefix.unprefixed(f).is_none());
    relocate(&files, prefix, Direction::Sync, dry_run)
}

/// Moves prefixed encrypted secrets and metadata back next to their decrypted paths.
pub fn prefix_revert(prefix: &PathPrefix, dry_run: bool) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut files = vec![];
    for dir in prefix
        .encrypted_dir()
        .into_iter()
        .chain(prefix.metadata_dir())
    {
        if dir.is_dir() {
            collect_files(dir, false, &mut files);
        }
    }
    files.sort();
    files.dedup();
    let files: Vec<_> = files
        .into_iter()
        .filter(|f| prefix.unprefixed(f).is_some())
        .collect();
    let moves = relocate(&files, prefix, Direction::Revert, dry_run)?;
    if !dry_run {
        for dir in prefix
            .encrypted_dir()
            .into_iter()
            .chain(prefix.metadata_dir())
        {
            prune_empty_dirs(dir);
        }
    }
    Ok(moves)
}
