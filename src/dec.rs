use crate::{
    Identity, OperationKind, OperationResult, project::append_to_gitignore_if_absent,
    to_decrypted_path,
};
use crate::{RecipientData, VerifyOptions, iter_encrypted, verify_file};
use age::armor::ArmoredReader;
use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::{Context, Result};
use filetime::{FileTime, set_file_mtime};
use std::io::Read;
use std::path::Path;

pub struct DecryptOptions {
    pub identities: Vec<Identity>,
    pub recipients: Vec<RecipientData>,
    pub dry_run: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_verify_encrypted: bool,
    pub skip_verify_recipients: bool,
}

pub fn decrypt_into_memory(
    input_reader: impl Read,
    opts: &DecryptOptions,
) -> Result<SecretSlice<u8>> {
    let decryptor = age::Decryptor::new_buffered(ArmoredReader::new(input_reader))?;
    let mut buffer = vec![];

    let mut decrypted = {
        let age_identities: Vec<Box<dyn age::Identity>> =
            opts.identities.iter().map(|id| id.clone().into()).collect();
        decryptor.decrypt(age_identities.iter().map(|id| id.as_ref()))
    }?;

    std::io::copy(&mut decrypted, &mut buffer)?;
    Ok(buffer.into())
}

pub fn decrypt_file(path: &Path, opts: &DecryptOptions) -> Result<Option<OperationResult>> {
    log::debug!("{}: decrypting file", path.display());
    // Just read operations for now ------------------------

    let verify_opts = VerifyOptions {
        recipients: opts.recipients.clone(),
        skip_verify_encrypted: opts.skip_verify_encrypted,
        skip_verify_recipients: opts.skip_verify_recipients,
    };

    let Some(verified) = verify_file(path, &verify_opts)? else {
        return Ok(None);
    };

    let output_path = to_decrypted_path(path)
        .with_context(|| format!("{}: could not determine output path", path.display()))?;
    let output = decrypt_into_memory(verified.content.as_slice(), opts)?;

    // Write starts here ------------------------

    let res = if output_path.exists()
        && std::fs::read(&output_path)? == output.expose_secret().to_vec()
    {
        log::debug!("{}: skipping write: content matches", output_path.display());
        None
    } else {
        let mut edits = vec![];
        // First add to .gitignore before creating the decrypted file, so that if the operation fails
        // for some reason, we won't have a decrypted file that is not ignored.
        if !opts.skip_gitignore
            && let Some(gi) = append_to_gitignore_if_absent(&output_path, opts.dry_run)?
        {
            edits.push(gi);
        };

        if !opts.dry_run {
            log::debug!("{}: writing decrypted file", output_path.display());
            std::fs::write(&output_path, output.expose_secret()).with_context(|| {
                format!("{}: could not write decrypted file", output_path.display())
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&output_path, std::fs::Permissions::from_mode(0o600))?;
                log::debug!("{}: set permissions to 600", output_path.display());
            }
        } else {
            log::debug!("{}: dry run: skipping write", output_path.display());
        }

        Some(OperationResult {
            kind: OperationKind::Decrypt,
            input: path.to_path_buf(),
            output: Some(output_path.clone()),
            edits,
            cleanups: vec![],
        })
    };
    if !opts.dry_run && !opts.skip_timestamps && output_path.exists() {
        log::debug!("{}: updating timestamp", output_path.display());
        set_file_mtime(&output_path, FileTime::from_system_time(verified.mtime))?;
    };
    Ok(res)
}

pub fn decrypt_dir(
    path: &Path,
    options: &DecryptOptions,
) -> impl Iterator<Item = Result<OperationResult>> {
    iter_encrypted(path).flat_map(|e| decrypt_file(e.path(), options).transpose())
}

pub fn decrypt_path<'a>(
    path: &'a Path,
    options: &'a DecryptOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    if path.is_dir() {
        Box::new(decrypt_dir(path, options))
    } else if path.is_file() {
        Box::new(decrypt_file(path, options).transpose().into_iter())
    } else {
        Box::new(std::iter::once(Err(anyhow::anyhow!(
            "{}: path does not exist",
            path.display()
        ))))
    }
}
