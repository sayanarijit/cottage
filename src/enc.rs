use crate::{
    ChecksumMetadata, DecryptOptions, DecryptionMode, Metadata, OperationKind, OperationResult,
    SecretMetadata, decrypt_into_memory, is_encrypted_path, project::append_to_gitignore_if_absent,
    to_decrypted_path, to_encrypted_path, to_metadata_path,
};
use crate::{RecipientData, generate_preview, is_metadata_path, make_checksum, verify_checksum};
use age::armor::ArmoredWriter;
use age::secrecy::{ExposeSecret, SecretSlice, SecretString};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum EncryptionMode {
    Passphrase(SecretString),
    Recipients(Vec<RecipientData>),
}

#[derive(Clone)]
pub struct EncryptOptions {
    pub mode: EncryptionMode,
    pub decryption_mode: Option<DecryptionMode>,
    pub identity_path: PathBuf,
    pub armor: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_preview: bool,
    pub skip_verify_recipients: bool,
    pub dry_run: bool,
}

fn build_matcher(globs: Option<&[Glob]>) -> Result<Option<GlobSet>> {
    let mut builder = GlobSetBuilder::new();
    if let Some(globs) = globs {
        for glob in globs {
            builder.add(glob.clone());
        }

        let res = builder.build()?;
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

fn filter_recipients_by_metadata<'a>(
    all_recipients: &'a [RecipientData],
    allow: Option<&'a [Glob]>,
    deny: Option<&'a [Glob]>,
) -> Result<Box<dyn Iterator<Item = &'a RecipientData> + 'a>> {
    let allowmatcher = build_matcher(allow)?;
    let denymatcher = build_matcher(deny)?;

    let filtered = all_recipients.into_iter().filter_map(move |r| {
        match (r.path.as_ref(), &allowmatcher, &denymatcher) {
            (Some(ref path), _, Some(deny)) if deny.is_match(path) => {
                log::debug!(
                    "{}: skipping recipient: path matches deny rules",
                    path.display()
                );
                None
            }
            (Some(ref path), Some(allow), _) if !allow.is_match(path) => {
                log::debug!(
                    "{}: skipping recipient: path does not match allow rules",
                    path.display()
                );
                None
            }
            _ => Some(r),
        }
    });

    Ok(Box::new(filtered))
}

pub fn encrypt_file(path: &Path, opts: &EncryptOptions) -> Result<Option<OperationResult>> {
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

    let input_file = File::open(path)
        .with_context(|| format!("{}: could not open input file", path.display()))?;
    log::debug!("{}: reading input file", path.display());

    let input = {
        let mut reader = BufReader::new(&input_file);
        let mut buffer = vec![];
        std::io::copy(&mut reader, &mut buffer)?;
        SecretSlice::new(buffer.into())
    };

    let output_path = to_encrypted_path(path);
    let metadata_path = to_metadata_path(path);
    let filemtime = input_file.metadata()?.modified()?;

    let (mut encryptor, mut recipients) = match &opts.mode {
        EncryptionMode::Passphrase(pass) => (
            age::Encryptor::with_user_passphrase(pass.clone()),
            crate::PASSPHRASE_RECIPIENT.as_bytes().to_vec(),
        ),
        EncryptionMode::Recipients(recipients) => (
            age::Encryptor::with_recipients(recipients.iter().map(|r| r.recipient.as_ref()))
                .map_err(|_| anyhow!("at least one recipient must be provided"))?,
            recipients
                .iter()
                .flat_map(|r| r.raw.iter().chain(&[b'\n']))
                .copied()
                .collect::<Vec<u8>>(),
        ),
    };

    let (old_content, old_metadata) = if output_path.exists() && metadata_path.exists() {
        log::debug!(
            "{}: existing encrypted and metadata files found",
            path.display()
        );

        let old_metadata = Metadata::read_from_path(&metadata_path).ok();

        match &opts.mode {
            EncryptionMode::Passphrase(_) => {
                log::debug!("passphrase mode: ignoring allow/deny rules in metadata");
            }
            EncryptionMode::Recipients(all_recipients) => {
                let allow = old_metadata
                    .as_ref()
                    .and_then(|m| m.secret.allow.as_ref())
                    .map(|globs| globs.as_slice());
                let deny = old_metadata
                    .as_ref()
                    .and_then(|m| m.secret.deny.as_ref())
                    .map(|globs| globs.as_slice());

                log::debug!(
                    "filtering recipients based on metadata rules: allow={:?}, deny={:?}",
                    allow.unwrap_or_default(),
                    deny.unwrap_or_default()
                );

                let filtered = filter_recipients_by_metadata(all_recipients, allow, deny)?;
                let (recps, raw) = filtered
                    .map(|r| (r.recipient.as_ref(), r.raw.iter().chain(&[b'\n'])))
                    .unzip::<_, _, Vec<_>, Vec<_>>();

                if recps.is_empty() {
                    return Err(anyhow!(
                        "{}: no recipients found to encrypt the secret for",
                        path.display()
                    ));
                }

                encryptor = age::Encryptor::with_recipients(recps.into_iter())?;
                recipients = raw.into_iter().flatten().copied().collect();
            }
        }

        if let (Some(dec_mode), Ok(f)) = (&opts.decryption_mode, File::open(&output_path)) {
            if !opts.skip_verify_recipients
                && let Some(metadata) = old_metadata.as_ref()
            {
                log::debug!(
                    "{}: verifying intended encryption recipients",
                    metadata_path.display()
                );
                verify_checksum(
                        &recipients.clone().into(),
                        &metadata.checksum.recipients,
                        &metadata_path,
                    )
                    .with_context(|| {
                        format!(
                            "{}: recipients mismatch, use --skip-verify-recipients or --force to overwrite",
                            metadata_path.display()
                        )
                    })?;
            }

            let decrypt_options = DecryptOptions {
                mode: dec_mode.clone(),
                recipients: recipients.clone(),
                skip_gitignore: true,
                skip_timestamps: true,
                skip_verify_encrypted: true,
                skip_verify_recipients: true,
                dry_run: true,
            };
            let content = decrypt_into_memory(f, &decrypt_options).ok();
            (content, old_metadata)
        } else {
            (None, old_metadata)
        }
    } else {
        (None, None)
    };

    let output = {
        let mut reader = BufReader::new(input.expose_secret());
        let mut buffer = vec![];
        let mut enc_writer =
            encryptor.wrap_output(ArmoredWriter::wrap_output(&mut buffer, format)?)?;
        std::io::copy(&mut reader, &mut enc_writer)?;
        enc_writer.finish().and_then(|armor| armor.finish())?;
        buffer.flush()?;
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
            old_content.as_ref(),
            old_metadata
                .as_ref()
                .and_then(|m| m.preview.as_ref().map(|p| p.preview.as_str())),
            &secret.timestamp,
        )
    } else {
        None
    };

    let checksum = {
        let encrypted = make_checksum(&output);
        ChecksumMetadata {
            encrypted,
            recipients: make_checksum(&recipients.clone().into()),
        }
    };

    let metadata = Metadata {
        secret,
        checksum,
        preview,
    };

    // Write starts here ------------------------

    if opts.dry_run {
        log::debug!(
            "{}: dry-run: skipping write of encrypted and metadata file",
            path.display()
        );
        Ok(None)
    } else {
        let res = if old_content
            .as_ref()
            .map(|c| c.expose_secret() == input.expose_secret())
            .unwrap_or(false)
        {
            log::debug!("{}: skipping write: content matches", output_path.display());
            None
        } else {
            // First add to .gitignore before creating the encrypted file, because, why not!
            let gitignore = if !opts.skip_gitignore {
                append_to_gitignore_if_absent(path, opts.dry_run)?
            } else {
                None
            };

            log::debug!("{}: writing encrypted file", output_path.display());
            std::fs::write(&output_path, output.expose_secret()).with_context(|| {
                format!("{}: could not write encrypted file", output_path.display())
            })?;
            Some(OperationResult {
                kind: OperationKind::Encrypt,
                input: path.to_path_buf(),
                output: output_path.clone(),
                gitignore,
                metadata: Some(metadata_path.clone()),
            })
        };

        if old_metadata.as_ref() == Some(&metadata) {
            log::debug!(
                "{}: skipping write of metadata file: metadata matches",
                metadata_path.display()
            );
        } else {
            log::debug!("{}: writing metadata file", metadata_path.display());
            std::fs::write(&metadata_path, toml::to_string(&metadata)?).with_context(|| {
                format!("{}: could not write metadata file", metadata_path.display())
            })?;
        }

        if !opts.skip_timestamps {
            log::debug!("{}: updating timestamp", output_path.display());
            set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
        }
        Ok(res)
    }
}

pub fn encrypt_dir(
    path: &Path,
    options: &EncryptOptions,
) -> impl Iterator<Item = Result<OperationResult>> {
    walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| is_encrypted_path(e.path()))
        .filter_map(|e| to_decrypted_path(e.path()))
        .filter_map(|path| encrypt_file(&path, options).transpose())
}

pub fn encrypt_path<'a>(
    path: &'a Path,
    options: &'a EncryptOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    if path.is_dir() {
        Box::new(encrypt_dir(path, options))
    } else if path.is_file() {
        Box::new(encrypt_file(path, options).transpose().into_iter())
    } else {
        Box::new(std::iter::once(Err(anyhow!(
            "{}: path does not exist",
            path.display()
        ))))
    }
}
