use std::{fs::File, io::BufReader, path::Path, time::SystemTime};

use age::secrecy::SecretSlice;
use anyhow::{Context, Result};

use crate::{
    Metadata, RecipientData, filter_recipients_by_metadata, is_encrypted_path, iter_encrypted,
    make_recipients_checksum_data, to_decrypted_path, to_metadata_path, verify_checksum,
};

pub struct VerifyOptions {
    pub recipients: Vec<RecipientData>,
    pub skip_verify_encrypted: bool,
    pub skip_verify_recipients: bool,
}

pub struct VerificationResult {
    pub content: Vec<u8>,
    pub mtime: SystemTime,
    pub recipients: Vec<RecipientData>,
}

pub fn verify_recipients(
    checksum: impl Into<SecretSlice<u8>>,
    path: &Path,
    metadata_path: &Path,
    metadata: &Metadata,
) -> Result<()> {
    verify_checksum(&checksum.into(), &metadata.checksum.recipients, path).with_context(|| {
        format!(
            "{}: recipients mismatch: use --skip-verify-recipients to skip this check",
            metadata_path.display()
        )
    })
}

pub fn verify_encrypted(
    input: impl Into<SecretSlice<u8>>,
    path: &Path,
    metadata_path: &Path,
    metadata: &Metadata,
) -> Result<()> {
    verify_checksum(&input.into(), &metadata.checksum.encrypted, path).with_context(|| {
        format!(
            "{}: content mismatch: use --skip-verify-encrypted to skip this check",
            metadata_path.display()
        )
    })
}

pub fn verify_file(path: &Path, opts: &VerifyOptions) -> Result<Option<VerificationResult>> {
    log::debug!("{}: verifying file", path.display());
    if !is_encrypted_path(path) {
        log::warn!("skipped: {}: invalid encrypted file", path.display());
        return Ok(None);
    }

    let dec_path = to_decrypted_path(path)
        .with_context(|| format!("{}: could not determine output path", path.display()))?;
    let metadata_path = to_metadata_path(&dec_path);
    let metadata = Metadata::read_from_path(&metadata_path)
        .with_context(|| format!("{}: could not read metadata", metadata_path.display()))?;

    log::debug!("{}: reading encrypted file", path.display());

    let input_file = File::open(path)
        .with_context(|| format!("{}: could not open input file", path.display()))?;
    let mtime = input_file.metadata()?.modified()?;

    let content = {
        let mut reader = BufReader::new(&input_file);
        let mut buffer = vec![];
        std::io::copy(&mut reader, &mut buffer)?;
        buffer
    };

    let allow = metadata.secret.allow.as_deref();
    let deny = metadata.secret.deny.as_deref();

    log::debug!(
        "filtering recipients based on metadata rules: allow={:?}, deny={:?}",
        allow.unwrap_or_default(),
        deny.unwrap_or_default()
    );

    let recipients =
        filter_recipients_by_metadata(&opts.recipients, allow, deny)?.collect::<Vec<_>>();
    let checksum_data = make_recipients_checksum_data(&recipients);

    if !opts.skip_verify_recipients {
        verify_recipients(checksum_data, path, &metadata_path, &metadata)?;
    }

    if !opts.skip_verify_encrypted {
        verify_encrypted(content.clone(), path, &metadata_path, &metadata)?;
    }

    Ok(Some(VerificationResult {
        content,
        mtime,
        recipients,
    }))
}

pub fn verify_dir(
    path: &Path,
    options: &VerifyOptions,
) -> impl Iterator<Item = Result<VerificationResult>> {
    iter_encrypted(path).filter_map(move |e| verify_file(e.path(), options).transpose())
}

pub fn verify_path<'a>(
    path: &'a Path,
    options: &'a VerifyOptions,
) -> Box<dyn Iterator<Item = Result<VerificationResult>> + 'a> {
    if path.is_dir() {
        Box::new(verify_dir(path, options))
    } else if path.is_file() {
        Box::new(verify_file(path, options).transpose().into_iter())
    } else {
        Box::new(std::iter::once(Err(anyhow::anyhow!(
            "{}: path does not exist",
            path.display()
        ))))
    }
}
