use crate::{
    is_encrypted_path, remove_from_gitignore_if_present, to_decrypted_path, to_encrypted_path,
};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct CleanOptions {
    pub dry_run: bool,
    pub gitignore: bool,
}

pub fn clean_file(path: PathBuf, opts: &CleanOptions) -> Result<Option<PathBuf>> {
    if !to_encrypted_path(&path).exists() {
        log::warn!("skipped: {}: not a secret", path.display());
        Ok(None)
    } else {
        log::debug!("{}: removing file", path.display());
        if !opts.dry_run {
            fs::remove_file(&path)?;
            if opts.gitignore {
                while (remove_from_gitignore_if_present(&path)?).is_some() {}
            }
        }
        Ok(Some(path))
    }
}

pub fn clean_dir(path: &Path, opts: &CleanOptions) -> impl Iterator<Item = Result<PathBuf>> {
    Box::new(
        walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| is_encrypted_path(e.path()))
            .filter_map(|e| to_decrypted_path(e.path()))
            .filter(|p| p.exists())
            .filter_map(move |p| clean_file(p, opts).transpose()),
    )
}

pub fn clean_path<'a>(
    path: &'a Path,
    opts: &'a CleanOptions,
) -> Box<dyn Iterator<Item = Result<PathBuf>> + 'a> {
    if path.is_file() {
        Box::new(clean_file(path.to_path_buf(), opts).transpose().into_iter())
    } else if path.is_dir() {
        Box::new(clean_dir(path, opts))
    } else {
        Box::new(std::iter::empty())
    }
}
