use crate::{
    DecryptOptions, DecryptionMode, EncryptOptions, EncryptionMode, Operation, OperationKind,
    OperationResult, dec::decrypt_file, enc::encrypt_file, is_encrypted_path, to_decrypted_path,
    to_encrypted_path,
};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct SyncOptions<'e, 'd> {
    pub encryption_mode: EncryptionMode<'e>,
    pub decryption_mode: DecryptionMode<'d>,
    pub armor: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_preview: bool,
}

pub fn status_file(path: &Path) -> Result<Option<Operation>> {
    let (encrypted_path, decrypted_path) = if is_encrypted_path(path) {
        let decrypted_path = to_decrypted_path(path)
            .ok_or_else(|| anyhow!("Failed to determine decrypted path for {:?}", path))?;
        (path.to_path_buf(), decrypted_path)
    } else {
        let encrypted_path = to_encrypted_path(path);
        (encrypted_path, path.to_path_buf())
    };

    if !encrypted_path.exists() && decrypted_path.exists() {
        Ok(Some(Operation {
            kind: OperationKind::Encrypt,
            input: decrypted_path,
            output: encrypted_path,
        }))
    } else if encrypted_path.exists() && !decrypted_path.exists() {
        Ok(Some(Operation {
            kind: OperationKind::Decrypt,
            input: encrypted_path,
            output: decrypted_path,
        }))
    } else {
        let encrypted_mtime = fs::metadata(&encrypted_path)?.modified()?;
        let decrypted_mtime = fs::metadata(&decrypted_path)?.modified()?;

        if encrypted_mtime > decrypted_mtime {
            Ok(Some(Operation {
                kind: OperationKind::Decrypt,
                input: encrypted_path,
                output: decrypted_path,
            }))
        } else if decrypted_mtime > encrypted_mtime {
            Ok(Some(Operation {
                kind: OperationKind::Encrypt,
                input: decrypted_path,
                output: encrypted_path,
            }))
        } else {
            Ok(None)
        }
    }
}

pub fn status_dir(path: &Path) -> impl Iterator<Item = Result<Operation>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_encrypted_path(e.path()))
        .filter_map(|e| status_file(e.path()).transpose())
}

pub fn status_path(path: &Path) -> Box<dyn Iterator<Item = Result<Operation>> + '_> {
    if path.is_file() {
        Box::new(status_file(path).transpose().into_iter())
    } else if path.is_dir() {
        Box::new(status_dir(path))
    } else {
        Box::new(std::iter::empty())
    }
}

fn perform(operation: &Operation, sync_options: &SyncOptions) -> Result<Option<OperationResult>> {
    match operation.kind {
        OperationKind::Encrypt => {
            let encrypt_options = EncryptOptions {
                mode: sync_options.encryption_mode.clone(),
                armor: sync_options.armor,
                skip_gitignore: sync_options.skip_gitignore,
                skip_timestamps: sync_options.skip_timestamps,
                skip_preview: sync_options.skip_preview,
                skip_checksum: false,
            };
            encrypt_file(&operation.input, &encrypt_options)
        }
        OperationKind::Decrypt => {
            let decrypt_options = DecryptOptions {
                mode: sync_options.decryption_mode.clone(),
                skip_gitignore: sync_options.skip_gitignore,
                skip_timestamps: sync_options.skip_timestamps,
                skip_checksum_encrypted: false,
                skip_checksum_decrypted: false,
            };
            decrypt_file(&operation.input, &decrypt_options)
        }
    }
}

pub fn sync_path<'a>(
    path: &'a Path,
    sync_options: &'a SyncOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    Box::new(
        status_path(path)
            .filter_map(move |res| res.and_then(|op| perform(&op, sync_options)).transpose()),
    )
}
