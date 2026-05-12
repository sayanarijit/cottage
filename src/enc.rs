use crate::{
    ChecksumMetadata, Metadata, OperationKind, OperationResult, SecretMetadata, is_encrypted_path,
    make_recipients_checksum_data, project::append_to_gitignore_if_absent, to_decrypted_path,
    to_encrypted_path, to_metadata_path,
};
use crate::{
    DecryptOptions, Identity, RecipientData, VerifyOptions, decrypt_into_memory, generate_preview,
    is_metadata_path, iter_encrypted, make_checksum, verify_file,
};
use age::armor::ArmoredWriter;
use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone)]
pub struct EncryptOptions {
    pub recipients: Vec<RecipientData>,
    pub identities: Vec<Identity>,
    pub identity_path: PathBuf,
    pub armor: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_preview: bool,
    pub force: bool,
    pub dry_run: bool,
}

pub fn encrypt_file(
    path: &Path,
    opts: &EncryptOptions,
    data: Option<SecretSlice<u8>>,
) -> Result<Option<OperationResult>> {
    log::debug!("{}: encrypting file", path.display());
    let is_identity = (|| {
        let p = path.canonicalize().ok()?;
        let i = opts.identity_path.canonicalize().ok()?;
        Some(p.starts_with(i))
    })()
    .unwrap_or(false);

    if is_encrypted_path(path) || is_metadata_path(path) || is_identity {
        log::warn!("skipped: {}: invalid path for encryption", path.display());
        return Ok(None);
    }

    // Just read operations for now ------------------------
    let format = if opts.armor {
        age::armor::Format::AsciiArmor
    } else {
        age::armor::Format::Binary
    };

    let (input, filemtime) = if let Some(data) = data {
        (data, SystemTime::now())
    } else {
        let input_file = File::open(path)
            .with_context(|| format!("{}: could not open input file", path.display()))?;
        log::debug!("{}: reading input file", path.display());
        let mut reader = BufReader::new(&input_file);
        let mut buffer = vec![];
        std::io::copy(&mut reader, &mut buffer)?;
        let data = SecretSlice::new(buffer.into());
        let filemtime = input_file.metadata()?.modified()?;
        (data, filemtime)
    };

    let output_path = to_encrypted_path(path);
    let metadata_path = to_metadata_path(path);

    let old_metadata = Metadata::read_from_path(&metadata_path).ok();
    let verified_data = if !opts.force && metadata_path.exists() && output_path.exists() {
        let verify_opts = VerifyOptions {
            recipients: opts.recipients.clone(),
            skip_verify_encrypted: true,
            skip_verify_recipients: true,
        };
        verify_file(&output_path, &verify_opts)
    } else {
        Ok(None)
    }?;

    let (old_secret, recipients) = if let Some(res) = verified_data {
        let dec_opts = DecryptOptions {
            identities: opts.identities.clone(),
            recipients: opts.recipients.clone(),
            dry_run: true, // No need to write decrypted content to disk
            skip_gitignore: true,
            skip_timestamps: true,
            skip_verify_encrypted: true,
            skip_verify_recipients: true,
        };
        let old_secret = decrypt_into_memory(res.content.as_slice(), &dec_opts)?;
        (Some(old_secret), res.recipients)
    } else {
        (None, opts.recipients.clone())
    };

    if recipients.is_empty() {
        return Err(anyhow!(
            "{}: no recipients found to encrypt the secret for",
            path.display()
        ));
    }

    let encryptor =
        age::Encryptor::with_recipients(recipients.iter().map(|r| r.recipient.as_ref()))?;

    let output = {
        let mut reader = BufReader::new(input.expose_secret());
        let mut buffer = vec![];
        let mut enc_writer =
            encryptor.wrap_output(ArmoredWriter::wrap_output(&mut buffer, format)?)?;
        std::io::copy(&mut reader, &mut enc_writer)?;
        enc_writer.finish().and_then(|armor| armor.finish())?;
        SecretSlice::new(buffer.into())
    };

    let timestamp = DateTime::<Utc>::from(filemtime).to_rfc3339();
    let allow = old_metadata
        .as_ref()
        .and_then(|m| m.secret.allow.as_ref().map(|s| s.to_vec()));
    let deny = old_metadata
        .as_ref()
        .and_then(|m| m.secret.deny.as_ref().map(|s| s.to_vec()));

    let secret = SecretMetadata {
        timestamp,
        allow,
        deny,
    };

    let preview = if !opts.skip_preview {
        generate_preview(
            path,
            &input,
            old_secret.as_ref(),
            old_metadata
                .as_ref()
                .and_then(|m| m.preview.as_ref().map(|p| p.preview.as_str())),
            &secret.timestamp,
        )
    } else {
        None
    };

    let recp_checksum = make_recipients_checksum_data(&recipients);
    let recp_checksum = make_checksum(&recp_checksum.into());
    let checksum = {
        let encrypted = make_checksum(&output);
        ChecksumMetadata {
            encrypted,
            recipients: recp_checksum.clone(),
        }
    };

    let metadata = Metadata {
        secret,
        checksum,
        preview,
        upstream: old_metadata
            .as_ref()
            .map(|m| m.upstream.clone())
            .unwrap_or_default(),
    };

    // Write starts here ------------------------

    let recp_matches = old_metadata
        .as_ref()
        .map(|m| m.checksum.recipients == recp_checksum)
        .unwrap_or(false);

    let res = if recp_matches
        && old_secret
            .as_ref()
            .map(|c| c.expose_secret() == input.expose_secret())
            .unwrap_or(false)
    {
        log::debug!(
            "{}: skipping write: content and recipient matches",
            output_path.display()
        );
        None
    } else {
        let mut edits = Vec::new();
        // First add to .gitignore before creating the encrypted file, because, why not!
        if !opts.skip_gitignore
            && let Some(gi) = append_to_gitignore_if_absent(path, opts.dry_run)?
        {
            edits.push(gi);
        }

        if !opts.dry_run {
            log::debug!("{}: writing encrypted file", output_path.display());
            std::fs::write(&output_path, output.expose_secret()).with_context(|| {
                format!("{}: could not write encrypted file", output_path.display())
            })?;

            log::debug!("{}: writing metadata file", metadata_path.display());
            std::fs::write(&metadata_path, toml::to_string(&metadata)?).with_context(|| {
                format!("{}: could not write metadata file", metadata_path.display())
            })?;
        } else {
            log::debug!("{}: dry run: skipping write", output_path.display());
        }

        edits.push(metadata_path);

        Some(OperationResult {
            kind: OperationKind::Encrypt,
            input: path.to_path_buf(),
            output: Some(output_path.clone()),
            edits,
            cleanups: vec![],
        })
    };

    if !opts.dry_run && !opts.skip_timestamps && output_path.exists() {
        log::debug!("{}: updating timestamp", output_path.display());
        set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
    }
    Ok(res)
}

pub fn encrypt_dir(
    path: &Path,
    options: &EncryptOptions,
) -> impl Iterator<Item = Result<OperationResult>> {
    iter_encrypted(path)
        .filter_map(|e| to_decrypted_path(e.path()))
        .filter_map(|path| encrypt_file(&path, options, None).transpose())
}

pub fn encrypt_path<'a>(
    path: &'a Path,
    options: &'a EncryptOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    if path.is_dir() {
        Box::new(encrypt_dir(path, options))
    } else if path.is_file() {
        Box::new(encrypt_file(path, options, None).transpose().into_iter())
    } else {
        Box::new(std::iter::once(Err(anyhow!(
            "{}: path does not exist",
            path.display()
        ))))
    }
}
