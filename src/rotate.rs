use crate::{
    DecryptOptions, EncryptOptions, OperationResult, PathPrefix, VerifyOptions,
    decrypt_into_memory, encrypt_file, resolve_encrypted_secret_path, verify_file,
};
use anyhow::{Context, Result};
use std::path::Path;

/// Re-encrypts the secret at `path` for the current recipients, entirely in memory.
///
/// The recipients checksum is not verified, as it is expected to change.
pub fn rotate(
    prefix: &PathPrefix,
    path: &Path,
    opts: &EncryptOptions,
) -> Result<Option<OperationResult>> {
    let encrypted_path = resolve_encrypted_secret_path(prefix, path)?;
    let decrypted_path = prefix
        .to_decrypted_path(&encrypted_path)
        .with_context(|| format!("{}: could not determine output path", path.display()))?;

    let verified = verify_file(
        prefix,
        &encrypted_path,
        &VerifyOptions {
            recipients: opts.recipients.clone(),
            skip_verify_encrypted: opts.force,
            skip_verify_recipients: true,
        },
    )?
    .with_context(|| format!("{}: invalid encrypted secret", encrypted_path.display()))?;

    let decrypted = decrypt_into_memory(
        verified.content.as_slice(),
        &DecryptOptions {
            identities: opts.identities.clone(),
            recipients: verified.recipients.clone(),
            dry_run: false,
            skip_gitignore: true,
            skip_timestamps: true,
            skip_verify_encrypted: true,
            skip_verify_recipients: true,
        },
    )?;

    // Recipients are already filtered by allow/deny rules; force skips the
    // (expected to fail) verification and encrypts for exactly these.
    let enc_opts = EncryptOptions {
        recipients: verified.recipients,
        force: true,
        ..opts.clone()
    };
    encrypt_file(prefix, &decrypted_path, &enc_opts, Some(decrypted)).map(|maybeop| {
        maybeop.map(|op| OperationResult {
            input: encrypted_path,
            ..op
        })
    })
}

/// Rotates a single secret, or every encrypted secret under a directory.
pub fn rotate_path<'a>(
    prefix: &'a PathPrefix,
    path: &'a Path,
    opts: &'a EncryptOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    if path.is_dir() {
        Box::new(
            prefix
                .iter_encrypted(path)
                .filter_map(|e| rotate(prefix, e.path(), opts).transpose()),
        )
    } else {
        Box::new(rotate(prefix, path, opts).transpose().into_iter())
    }
}
