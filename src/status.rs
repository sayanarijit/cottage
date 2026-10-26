use crate::{Operation, OperationKind, PathPrefix, is_encrypted_path, is_metadata_path};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct StatusOptions {
    pub skip_encryption: bool,
    pub skip_decryption: bool,
}

pub fn status_file(
    prefix: &PathPrefix,
    path: &Path,
    opts: StatusOptions,
) -> Result<Option<Operation>> {
    if is_metadata_path(path) {
        return Ok(None);
    }

    let (encrypted_path, decrypted_path) = if is_encrypted_path(path) {
        let decrypted_path = prefix
            .to_decrypted_path(path)
            .ok_or_else(|| anyhow!("{}: could not determine decrypted path", path.display()))?;
        (path.to_path_buf(), decrypted_path)
    } else {
        let encrypted_path = prefix.to_encrypted_path(path);
        (encrypted_path, path.to_path_buf())
    };

    if !encrypted_path.exists() && decrypted_path.exists() {
        if opts.skip_encryption {
            log::debug!(
                "{}: status: encryption required (skipped)",
                decrypted_path.display()
            );
            Ok(None)
        } else {
            log::debug!("{}: status: encryption required", decrypted_path.display());
            Ok(Some(Operation {
                kind: OperationKind::Encrypt,
                input: decrypted_path,
                output: encrypted_path,
            }))
        }
    } else if encrypted_path.exists() && !decrypted_path.exists() {
        if opts.skip_decryption {
            log::debug!(
                "{}: status: decryption required (skipped)",
                encrypted_path.display()
            );
            Ok(None)
        } else {
            log::debug!("{}: status: decryption required", encrypted_path.display());
            Ok(Some(Operation {
                kind: OperationKind::Decrypt,
                input: encrypted_path,
                output: decrypted_path,
            }))
        }
    } else {
        let encrypted_mtime = fs::metadata(&encrypted_path)?.modified()?;
        let decrypted_mtime = fs::metadata(&decrypted_path)?.modified()?;

        if encrypted_mtime > decrypted_mtime {
            if opts.skip_decryption {
                log::debug!(
                    "{}: status: decryption required (encrypted file is newer, skipped)",
                    encrypted_path.display()
                );
                Ok(None)
            } else {
                log::debug!(
                    "{}: status: decryption required (encrypted file is newer)",
                    encrypted_path.display()
                );
                Ok(Some(Operation {
                    kind: OperationKind::Decrypt,
                    input: encrypted_path,
                    output: decrypted_path,
                }))
            }
        } else if decrypted_mtime > encrypted_mtime {
            if opts.skip_encryption {
                log::debug!(
                    "{}: status: encryption required (decrypted file is newer, skipped)",
                    decrypted_path.display()
                );
                Ok(None)
            } else {
                log::debug!(
                    "{}: status: encryption required (decrypted file is newer)",
                    decrypted_path.display()
                );
                Ok(Some(Operation {
                    kind: OperationKind::Encrypt,
                    input: decrypted_path,
                    output: encrypted_path,
                }))
            }
        } else {
            log::debug!("{}: status: up to date", decrypted_path.display());
            Ok(None)
        }
    }
}

pub fn status_dir<'a>(
    prefix: &'a PathPrefix,
    path: &Path,
    opts: StatusOptions,
) -> impl Iterator<Item = Result<Operation>> + 'a {
    prefix
        .iter_encrypted(path)
        .filter_map(move |e| status_file(prefix, e.path(), opts).transpose())
}

pub fn status_path<'a>(
    prefix: &'a PathPrefix,
    path: &'a Path,
    opts: StatusOptions,
) -> Box<dyn Iterator<Item = Result<Operation>> + 'a> {
    if path.is_file() {
        Box::new(status_file(prefix, path, opts).transpose().into_iter())
    } else if path.is_dir() {
        Box::new(status_dir(prefix, path, opts))
    } else {
        Box::new(std::iter::once(Err(anyhow!(
            "{}: path does not exist",
            path.display()
        ))))
    }
}
