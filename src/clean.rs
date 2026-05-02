use crate::{
    Project, is_encrypted_path, remove_from_gitignore_if_present, to_decrypted_path,
    to_encrypted_path,
};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct CleanOptions {
    pub dry_run: bool,
    pub skip_gitignore: bool,
}

pub fn clean_file(
    path: PathBuf,
    opts: &CleanOptions,
    is_proj_file: bool,
) -> Result<Option<PathBuf>> {
    if !is_proj_file && !to_encrypted_path(&path).exists() {
        log::warn!("skipped: {}: not a secret", path.display());
        Ok(None)
    } else {
        log::debug!("{}: removing file", path.display());
        if !opts.dry_run {
            fs::remove_file(&path)?;
            if !opts.skip_gitignore {
                while (remove_from_gitignore_if_present(&path)?).is_some() {
                    //
                }
            }
        }
        Ok(Some(path))
    }
}

pub fn clean_dir(
    path: &Path,
    opts: &CleanOptions,
    is_proj_file: bool,
) -> impl Iterator<Item = Result<PathBuf>> {
    Box::new(
        walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| is_encrypted_path(e.path()))
            .filter_map(|e| to_decrypted_path(e.path()))
            .filter(|p| p.exists())
            .filter_map(move |p| clean_file(p, opts, is_proj_file).transpose()),
    )
}

pub fn clean_path<'a>(
    path: &'a Path,
    opts: &'a CleanOptions,
    is_proj_file: bool,
) -> Box<dyn Iterator<Item = Result<PathBuf>> + 'a> {
    if path.is_file() {
        Box::new(
            clean_file(path.to_path_buf(), opts, is_proj_file)
                .transpose()
                .into_iter(),
        )
    } else if path.is_dir() {
        Box::new(clean_dir(path, opts, is_proj_file))
    } else {
        Box::new(std::iter::empty())
    }
}

pub fn clean_project<'a>(
    proj: &'a Project,
    opts: &'a CleanOptions,
) -> Box<dyn Iterator<Item = Result<PathBuf>> + 'a> {
    Box::new(clean_path(proj.root(), opts, false).chain(clean_path(
        proj.identity_path(),
        opts,
        true,
    )))
}
