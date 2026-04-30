use crate::dec::decrypt_file;
use crate::enc::encrypt_file;
use crate::{
    DecryptOptions, DecryptionMode, EncryptOptions, EncryptionMode, is_encrypted_path,
    to_decrypted_path, to_encrypted_path,
};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

pub struct SyncOptions<'e, 'd> {
    pub encryption_mode: EncryptionMode<'e>,
    pub decryption_mode: DecryptionMode<'d>,
    pub armor: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
}

#[derive(Debug, Clone)]
pub enum PendingOperation {
    Encrypt(PathBuf, PathBuf),
    Decrypt(PathBuf, PathBuf),
}

pub fn status_file(path: &Path) -> Result<Option<PendingOperation>> {
    let (encrypted_path, decrypted_path) = if is_encrypted_path(path) {
        let decrypted_path = to_decrypted_path(path)
            .ok_or_else(|| anyhow!("Failed to determine decrypted path for {:?}", path))?;
        (path.to_path_buf(), decrypted_path)
    } else {
        let encrypted_path = to_encrypted_path(path);
        (encrypted_path, path.to_path_buf())
    };

    if !encrypted_path.exists() && decrypted_path.exists() {
        Ok(Some(PendingOperation::Encrypt(
            decrypted_path,
            encrypted_path,
        )))
    } else if encrypted_path.exists() && !decrypted_path.exists() {
        Ok(Some(PendingOperation::Decrypt(
            encrypted_path,
            decrypted_path,
        )))
    } else {
        let encrypted_mtime = fs::metadata(path)?.modified()?;
        let decrypted_mtime = fs::metadata(&decrypted_path)?.modified()?;

        if encrypted_mtime > decrypted_mtime {
            Ok(Some(PendingOperation::Decrypt(
                encrypted_path,
                decrypted_path,
            )))
        } else if decrypted_mtime > encrypted_mtime {
            Ok(Some(PendingOperation::Encrypt(
                decrypted_path,
                encrypted_path,
            )))
        } else {
            Ok(None)
        }
    }
}

pub fn status_dir(path: &Path) -> impl Iterator<Item = Result<PendingOperation>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_encrypted_path(e.path()))
        .filter_map(|e| status_file(e.path()).transpose())
}

pub fn status_path(path: &Path) -> Box<dyn Iterator<Item = Result<PendingOperation>> + '_> {
    if path.is_file() {
        Box::new(status_file(path).transpose().into_iter())
    } else if path.is_dir() {
        Box::new(status_dir(path))
    } else {
        Box::new(std::iter::empty())
    }
}

pub fn perform(
    operation: &PendingOperation,
    sync_options: &SyncOptions,
) -> Result<Option<(PathBuf, PathBuf, Option<PathBuf>)>> {
    match operation {
        PendingOperation::Encrypt(src, _) => {
            let encrypt_options = EncryptOptions {
                mode: sync_options.encryption_mode.clone(),
                armor: sync_options.armor,
                skip_gitignore: sync_options.skip_gitignore,
                skip_timestamps: sync_options.skip_timestamps,
            };
            encrypt_file(src, &encrypt_options)
                .map(|(p1, p2, p3)| (p1.to_path_buf(), p2, p3))
                .map(Some)
        }
        PendingOperation::Decrypt(src, _) => {
            let decrypt_options = DecryptOptions {
                mode: sync_options.decryption_mode.clone(),
                skip_gitignore: sync_options.skip_gitignore,
                skip_timestamps: sync_options.skip_timestamps,
            };
            decrypt_file(src, &decrypt_options)
                .map(|(p1, p2, p3)| (p1.to_path_buf(), p2, p3))
                .map(Some)
        }
    }
}

pub fn sync_path<'a>(
    path: &'a Path,
    sync_options: &'a SyncOptions,
) -> Box<dyn Iterator<Item = Result<(PathBuf, PathBuf, Option<PathBuf>)>> + 'a> {
    Box::new(
        status_path(path)
            .map(move |res| res.and_then(|op| perform(&op, sync_options)))
            .filter_map(|res| res.transpose()),
    )
}
