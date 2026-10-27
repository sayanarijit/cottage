use crate::PathPrefix;
use crate::{
    DecryptOptions, EncryptOptions, OperationResult, VerifyOptions, decrypt_into_memory,
    is_encrypted_path, verify_file,
};
use age::secrecy::ExposeSecret;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct CatOptions {
    pub encrypt_options: EncryptOptions,
    pub skip_verify_encrypted: bool,
    pub skip_verify_recipients: bool,
}

pub fn resolve_encrypted_secret_path(prefix: &PathPrefix, path: &Path) -> Result<PathBuf> {
    if is_encrypted_path(path) {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        anyhow::bail!("{}: path does not exist", path.display());
    }

    let decrypted_path = prefix
        .to_decrypted_path(path)
        .unwrap_or_else(|| path.to_path_buf());
    let encrypted_path = prefix.to_encrypted_path(&decrypted_path);
    if encrypted_path.is_file() {
        Ok(encrypted_path)
    } else {
        anyhow::bail!(
            "{}: encrypted secret does not exist",
            encrypted_path.display()
        );
    }
}

/// Decrypts `path` into `out`.
pub fn cat(
    prefix: &PathPrefix,
    path: &Path,
    opts: &CatOptions,
    mut out: impl Write,
) -> Result<Option<OperationResult>> {
    let encrypted_path = resolve_encrypted_secret_path(prefix, path)?;

    let verify_options = VerifyOptions {
        recipients: opts.encrypt_options.recipients.clone(),
        skip_verify_encrypted: opts.encrypt_options.force || opts.skip_verify_encrypted,
        skip_verify_recipients: opts.encrypt_options.force || opts.skip_verify_recipients,
    };

    let verified = verify_file(prefix, &encrypted_path, &verify_options)?
        .with_context(|| format!("{}: invalid encrypted secret", encrypted_path.display()))?;

    let decrypted = decrypt_into_memory(
        verified.content.as_slice(),
        &DecryptOptions {
            identities: opts.encrypt_options.identities.clone(),
            recipients: verify_options.recipients,
            dry_run: false,
            skip_gitignore: true,
            skip_timestamps: true,
            skip_verify_encrypted: true,
            skip_verify_recipients: true,
        },
    )?;

    out.write_all(decrypted.expose_secret())?;
    out.flush()?;
    Ok(None)
}
