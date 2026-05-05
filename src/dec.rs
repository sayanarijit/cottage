use crate::{
    Identity, Metadata, OperationKind, OperationResult, is_encrypted_path,
    project::append_to_gitignore_if_absent, to_decrypted_path, to_metadata_path, verify_checksum,
};
use crate::{RecipientData, filter_recipients_by_metadata, make_recipients_checksum_data};
use age::armor::ArmoredReader;
use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::{Context, Result};
use filetime::{FileTime, set_file_mtime};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
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
    buffer.flush()?;
    Ok(buffer.into())
}

pub fn decrypt_file(path: &Path, opts: &DecryptOptions) -> Result<Option<OperationResult>> {
    log::debug!("{}: decrypting file", path.display());
    // Just read operations for now ------------------------
    if !is_encrypted_path(path) {
        log::warn!("skipped: {}: invalid path for decryption", path.display());
        return Ok(None);
    }

    let output_path = to_decrypted_path(path)
        .with_context(|| format!("{}: could not determine output path", path.display()))?;
    let metadata_path = to_metadata_path(&output_path);
    let metadata = Metadata::read_from_path(&metadata_path)
        .with_context(|| format!("{}: could not read metadata", metadata_path.display()))?;

    let mut input_file = File::open(path)
        .with_context(|| format!("{}: could not open input file", path.display()))?;
    log::debug!("{}: reading encrypted file", path.display());
    let filemtime = input_file.metadata()?.modified()?;

    let input = {
        let mut reader = BufReader::new(&input_file);
        let mut buffer = vec![];
        std::io::copy(&mut reader, &mut buffer)?;
        input_file.seek(SeekFrom::Start(0))?;
        buffer.flush()?;
        SecretSlice::new(buffer.into())
    };

    if !opts.skip_verify_recipients {
        let allow = metadata.secret.allow.as_deref();
        let deny = metadata.secret.deny.as_deref();

        log::debug!(
            "filtering recipients based on metadata rules: allow={:?}, deny={:?}",
            allow.unwrap_or_default(),
            deny.unwrap_or_default()
        );

        let filtered =
            filter_recipients_by_metadata(&opts.recipients, allow, deny)?.collect::<Vec<_>>();
        let checksum_data = make_recipients_checksum_data(&filtered);
        verify_checksum(&checksum_data.into(), &metadata.checksum.recipients, path).with_context(
            || {
                format!(
                    "{}: recipients mismatch: use --skip-verify-recipients to skip this check",
                    metadata_path.display()
                )
            },
        )?;
    }

    if !opts.skip_verify_encrypted {
        verify_checksum(&input, &metadata.checksum.encrypted, path).with_context(|| {
            format!(
                "{}: content mismatch: use --skip-verify-encrypted to skip this check",
                metadata_path.display()
            )
        })?;
    }

    let output = decrypt_into_memory(input_file, opts)?;

    // Write starts here ------------------------

    if opts.dry_run {
        log::debug!(
            "{}: dry-run: skipping write of decrypted file",
            output_path.display()
        );
        Ok(None)
    } else {
        let res = if output_path.exists()
            && std::fs::read(&output_path)? == output.expose_secret().to_vec()
        {
            log::debug!("{}: skipping write: content matches", output_path.display());
            None
        } else {
            // First add to .gitignore before creating the decrypted file, so that if the operation fails
            // for some reason, we won't have a decrypted file that is not ignored.
            let gitignorefile = if !opts.skip_gitignore {
                append_to_gitignore_if_absent(&output_path, opts.dry_run)?
            } else {
                None
            };

            log::debug!("{}: writing decrypted file", output_path.display());
            std::fs::write(&output_path, output.expose_secret()).with_context(|| {
                format!("{}: could not write decrypted file", output_path.display())
            })?;
            Some(OperationResult {
                kind: OperationKind::Decrypt,
                input: path.to_path_buf(),
                output: output_path.clone(),
                gitignore: gitignorefile,
                metadata: None,
            })
        };
        if !opts.skip_timestamps {
            log::debug!("{}: updating timestamp", output_path.display());
            set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
        };
        Ok(res)
    }
}

pub fn decrypt_dir(
    path: &Path,
    options: &DecryptOptions,
) -> impl Iterator<Item = Result<OperationResult>> {
    walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| is_encrypted_path(e.path()))
        .flat_map(|e| decrypt_file(e.path(), options).transpose())
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
