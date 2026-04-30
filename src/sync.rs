use crate::{
    DecryptOptions, DecryptionMode, EncryptOptions, EncryptionMode, decrypt_file, encrypt_file,
    is_encrypted_path, to_decrypted_path, to_encrypted_path,
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

pub fn sync_file(
    path: &Path,
    sync_options: &SyncOptions,
) -> Result<Option<(PathBuf, PathBuf, Option<PathBuf>)>> {
    let (encrypted_path, decrypted_path) = if is_encrypted_path(path) {
        let decrypted_path = to_decrypted_path(path)
            .ok_or_else(|| anyhow!("Failed to determine decrypted path for {:?}", path))?;
        (path.to_path_buf(), decrypted_path)
    } else {
        let encrypted_path = to_encrypted_path(path);
        (encrypted_path, path.to_path_buf())
    };

    let encrypt_options = EncryptOptions {
        mode: sync_options.encryption_mode.clone(),
        armor: sync_options.armor,
        skip_gitignore: sync_options.skip_gitignore,
        skip_timestamps: sync_options.skip_timestamps,
    };

    let decrypt_options = DecryptOptions {
        mode: sync_options.decryption_mode.clone(),
        skip_gitignore: sync_options.skip_gitignore,
        skip_timestamps: sync_options.skip_timestamps,
    };

    if !encrypted_path.exists() {
        Ok(Some(
            encrypt_file(&decrypted_path, &encrypt_options)
                .map(|(p1, p2, p3)| (p1.to_path_buf(), p2, p3))?,
        ))
    } else if !decrypted_path.exists() {
        Ok(Some(
            decrypt_file(&encrypted_path, &decrypt_options)
                .map(|(p1, p2, p3)| (p1.to_path_buf(), p2, p3))?,
        ))
    } else {
        let encrypted_mtime = fs::metadata(path)?.modified()?;
        let decrypted_mtime = fs::metadata(&decrypted_path)?.modified()?;

        if encrypted_mtime > decrypted_mtime {
            Ok(Some(
                decrypt_file(&encrypted_path, &decrypt_options)
                    .map(|(p1, p2, p3)| (p1.to_path_buf(), p2, p3))?,
            ))
        } else if decrypted_mtime > encrypted_mtime {
            Ok(Some(
                encrypt_file(&decrypted_path, &encrypt_options)
                    .map(|(p1, p2, p3)| (p1.to_path_buf(), p2, p3))?,
            ))
        } else {
            Ok(None)
        }
    }
}

pub fn sync_dir(
    path: &Path,
    sync_options: &SyncOptions,
) -> impl Iterator<Item = Result<(PathBuf, PathBuf, Option<PathBuf>)>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_encrypted_path(e.path()))
        .filter_map(|e| sync_file(e.path(), sync_options).transpose())
}
