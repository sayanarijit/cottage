use crate::{
    DecryptOptions, DecryptionMode, EncryptOptions, EncryptionMode, Operation, OperationKind,
    OperationResult, dec::decrypt_file, enc::encrypt_file, status::StatusOptions, status_path,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct SyncOptions {
    pub encryption_mode: EncryptionMode,
    pub identities: DecryptionMode,
    pub recipients: Vec<u8>,
    pub identity_path: PathBuf,
    pub armor: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_preview: bool,
    pub skip_encryption: bool,
    pub skip_decryption: bool,
    pub skip_verify_encrypted: bool,
    pub skip_verify_recipients: bool,
    pub force_encrypt: bool,
    pub dry_run: bool,
}

fn perform(operation: &Operation, sync_options: &SyncOptions) -> Result<Option<OperationResult>> {
    match operation.kind {
        OperationKind::Encrypt => {
            if !sync_options.skip_encryption {
                let encrypt_options = EncryptOptions {
                    decryption_mode: Some(sync_options.identities.clone()),
                    mode: sync_options.encryption_mode.clone(),
                    identity_path: sync_options.identity_path.clone(),
                    armor: sync_options.armor,
                    skip_gitignore: sync_options.skip_gitignore,
                    skip_timestamps: sync_options.skip_timestamps,
                    skip_preview: sync_options.skip_preview,
                    skip_verify_recipients: sync_options.force_encrypt
                        || sync_options.skip_verify_recipients,
                    dry_run: sync_options.dry_run,
                };
                encrypt_file(&operation.input, &encrypt_options)
            } else {
                log::debug!("{}: skipping encryption", operation.input.display());
                Ok(None)
            }
        }
        OperationKind::Decrypt => {
            if !sync_options.skip_decryption {
                let decrypt_options = DecryptOptions {
                    mode: sync_options.identities.clone(),
                    recipients: sync_options.recipients.clone(),
                    skip_gitignore: sync_options.skip_gitignore,
                    skip_timestamps: sync_options.skip_timestamps,
                    skip_verify_encrypted: sync_options.skip_verify_encrypted,
                    skip_verify_recipients: sync_options.skip_verify_recipients,
                    dry_run: sync_options.dry_run,
                };
                decrypt_file(&operation.input, &decrypt_options)
            } else {
                log::debug!("{}: skipping decryption", operation.input.display());
                Ok(None)
            }
        }
    }
}

pub fn sync_path<'a>(
    path: &'a Path,
    opts: &'a SyncOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    log::debug!("{}: syncing path", path.display());

    let status_opts = StatusOptions {
        skip_encryption: opts.skip_encryption,
        skip_decryption: opts.skip_decryption,
    };
    Box::new(
        status_path(path, status_opts)
            .filter_map(|res| res.and_then(|op| perform(&op, opts)).transpose()),
    )
}
