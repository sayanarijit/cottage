use crate::{is_encrypted_path, is_metadata_path, secure_remove_file, to_decrypted_path};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
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

thread_local! {
    static PATH_PREFIX: RefCell<PathPrefix> = RefCell::new(PathPrefix::default());
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

    /// Makes this prefix the one used by the path helpers on the current thread.
    pub fn activate(self) {
        PATH_PREFIX.with(|p| *p.borrow_mut() = self);
    }

    pub fn current() -> Self {
        PATH_PREFIX.with(|p| p.borrow().clone())
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

    pub fn encrypted_path(&self, path: &Path) -> PathBuf {
        self.under(self.encrypted_dir(), path)
            .unwrap_or_else(|| path.to_path_buf())
            .with_added_extension("cott")
            .with_added_extension("age")
    }

    pub fn metadata_path(&self, path: &Path) -> PathBuf {
        self.under(self.metadata_dir(), path)
            .unwrap_or_else(|| path.to_path_buf())
            .with_added_extension("cott")
            .with_added_extension("toml")
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
        let Some(decrypted) = to_decrypted_path(path) else {
            continue;
        };
        let target = match (direction, is_encrypted_path(path)) {
            (Direction::Sync, true) => prefix.encrypted_path(&decrypted),
            (Direction::Sync, false) => prefix.metadata_path(&decrypted),
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
pub fn prefix_sync(path: &Path, dry_run: bool) -> Result<Vec<(PathBuf, PathBuf)>> {
    let prefix = PathPrefix::current();
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
    relocate(&files, &prefix, Direction::Sync, dry_run)
}

/// Moves prefixed encrypted secrets and metadata back next to their decrypted paths.
pub fn prefix_revert(dry_run: bool) -> Result<Vec<(PathBuf, PathBuf)>> {
    let prefix = PathPrefix::current();
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
    let moves = relocate(&files, &prefix, Direction::Revert, dry_run)?;
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
