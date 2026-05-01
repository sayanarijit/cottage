use crate::{Project, is_encrypted_path, remove_from_gitignore_if_present, to_decrypted_path};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CleanOptions {
    pub dry_run: bool,
    pub skip_gitignore: bool,
}

pub fn clean_file(path: PathBuf, opts: &CleanOptions) -> Result<Option<PathBuf>> {
    if path.is_file() {
        if !opts.dry_run {
            fs::remove_file(&path)?;
            if !opts.skip_gitignore {
                while remove_from_gitignore_if_present(path.clone())?.is_some() {
                    // Keep trying to remove from gitignore until it returns false, in case there are multiple entries for the same file
                }
            }
        }
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

pub fn clean_path(path: &Path, opts: &CleanOptions) -> impl Iterator<Item = Result<PathBuf>> {
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

pub fn clean_project<'a>(
    proj: &'a Project,
    opts: &'a CleanOptions,
) -> Box<dyn Iterator<Item = Result<PathBuf>> + 'a> {
    Box::new(
        clean_path(proj.root(), opts).chain(
            clean_file(proj.identity_path().into(), opts)
                .transpose()
                .into_iter(),
        ),
    )
}
