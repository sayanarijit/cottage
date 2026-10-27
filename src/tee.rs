use crate::{EncryptOptions, OperationResult, PathPrefix, encrypt_file};
use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::{Context, Result};
use std::path::Path;

/// Encrypts `input` into `path`, returning the operation result.
pub fn tee(
    prefix: &PathPrefix,
    path: &Path,
    input: Option<SecretSlice<u8>>,
    opts: &EncryptOptions,
) -> Result<Option<OperationResult>> {
    let data = input
        .filter(|b| !b.expose_secret().is_empty())
        .context("expected content from stdin")?;
    let path = prefix
        .to_decrypted_path(path)
        .unwrap_or_else(|| path.to_path_buf());
    encrypt_file(prefix, &path, opts, Some(data))
}
