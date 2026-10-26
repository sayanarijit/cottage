use crate::{
    OperationKind, OperationResult, PathPrefix, is_encrypted_path, is_metadata_path,
    remove_from_gitignore_if_present, secure_remove_file,
};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default)]
pub struct CleanOptions {
    pub dry_run: bool,
    pub encrypted: bool,
    pub gitignore: bool,
}

pub fn clean_file(
    prefix: &PathPrefix,
    path: PathBuf,
    opts: &CleanOptions,
) -> Result<Option<OperationResult>> {
    let encrypted_path = prefix.to_encrypted_path(&path);
    if !encrypted_path.exists() {
        log::warn!("skipped: {}: not a secret", path.display());
        return Ok(None);
    }

    let mut was_cleaned = false;
    let mut cleanups = Vec::new();
    let mut edits = Vec::new();

    if path.exists() {
        if !opts.dry_run {
            log::debug!("{}: removing file", path.display());
            secure_remove_file(&path)?;
        } else {
            log::debug!("{}: would remove file (dry run)", path.display());
        }
        was_cleaned = true;
    }

    if opts.gitignore {
        while let Some(gi) = remove_from_gitignore_if_present(prefix, &path, opts.dry_run)? {
            was_cleaned = true;
            edits.push(gi);
            if opts.dry_run {
                break;
            }
        }
    }

    if opts.encrypted {
        if encrypted_path.exists() {
            was_cleaned = true;
            if !opts.dry_run {
                fs::remove_file(&encrypted_path)?;
                log::debug!("{}: removed encrypted file", path.display());
            } else {
                log::debug!("{}: would remove encrypted file (dry run)", path.display());
            }
            cleanups.push(encrypted_path);
        }

        let metadata_path = prefix.to_metadata_path(&path);
        if metadata_path.exists() {
            was_cleaned = true;
            if !opts.dry_run {
                fs::remove_file(&metadata_path)?;
                log::debug!("{}: removed metadata file", path.display());
            } else {
                log::debug!("{}: would remove metadata file (dry run)", path.display());
            }
            cleanups.push(metadata_path);
        }
    }

    if was_cleaned {
        Ok(Some(OperationResult {
            kind: OperationKind::Delete,
            input: path,
            output: None,
            cleanups,
            edits,
        }))
    } else {
        Ok(None)
    }
}

pub fn clean_dir<'a>(
    prefix: &'a PathPrefix,
    path: &Path,
    opts: &'a CleanOptions,
) -> impl Iterator<Item = Result<OperationResult>> + 'a {
    Box::new(
        prefix
            .iter_encrypted(path)
            .filter_map(|e| prefix.to_decrypted_path(e.path()))
            .filter_map(move |p| clean_file(prefix, p, opts).transpose()),
    )
}

pub fn clean_path<'a>(
    prefix: &'a PathPrefix,
    path: &'a Path,
    opts: &'a CleanOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    let decrypted_path = if is_encrypted_path(path) || is_metadata_path(path) {
        prefix
            .to_decrypted_path(path)
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    if decrypted_path.is_file() || prefix.to_encrypted_path(&decrypted_path).is_file() {
        Box::new(
            clean_file(prefix, decrypted_path, opts)
                .transpose()
                .into_iter(),
        )
    } else if path.is_dir() {
        Box::new(clean_dir(prefix, path, opts))
    } else {
        Box::new(std::iter::once(Err(anyhow::anyhow!(
            "{}: path does not exist",
            path.display()
        ))))
    }
}
