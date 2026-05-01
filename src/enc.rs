use crate::{
    ChecksumMetadata, DecryptOptions, DecryptionMode, Metadata, OperationKind, OperationResult,
    SecretMetadata, decrypt_into_memory, is_encrypted_path, project::append_to_gitignore_if_absent,
    to_decrypted_path, to_encrypted_path, to_metadata_path,
};
use crate::{generate_preview, is_metadata_path, make_checksum, validate_checksum};
use age::armor::ArmoredWriter;
use age::secrecy::SecretString;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use filetime::{FileTime, set_file_mtime};
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

#[derive(Clone)]
pub enum EncryptionMode<'a> {
    Passphrase(String),
    Recipients(&'a [(Box<dyn age::Recipient + Send>, Vec<u8>)]),
}

#[derive(Clone)]
pub struct EncryptOptions<'a> {
    pub mode: EncryptionMode<'a>,
    pub identities: &'a [Box<dyn age::Identity>],
    pub armor: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_preview: bool,
    pub skip_checksum: bool,
}

pub fn encrypt_file<'a>(
    path: &'a Path,
    options: &EncryptOptions,
) -> Result<Option<OperationResult>> {
    if is_encrypted_path(path) || is_metadata_path(path) {
        eprintln!(
            "{} {}: invalid path for encryption",
            "skipped:".yellow(),
            path.display()
        );
        return Ok(None);
    }

    // Just read operations for now ------------------------
    let (encryptor, recipients_data) = match &options.mode {
        EncryptionMode::Passphrase(pass) => (
            age::Encryptor::with_user_passphrase(SecretString::from(pass.as_str())),
            pass.as_bytes().to_vec(),
        ),
        EncryptionMode::Recipients(recipients) => (
            age::Encryptor::with_recipients(recipients.iter().map(|(r, _)| r.as_ref() as _))
                .map_err(|_| anyhow!("At least one recipient must be provided"))?,
            recipients
                .into_iter()
                .map(|(_, r)| r)
                .flatten()
                .map(|b| *b)
                .collect::<Vec<u8>>(),
        ),
    };

    let format = if options.armor {
        age::armor::Format::AsciiArmor
    } else {
        age::armor::Format::Binary
    };

    let input_file =
        File::open(path).with_context(|| format!("Failed to open input file: {:?}", path))?;

    let input = {
        let mut reader = BufReader::new(&input_file);
        let mut buffer = vec![];
        std::io::copy(&mut reader, &mut buffer)?;
        buffer
    };

    let output_path = to_encrypted_path(path);
    let metadata_path = to_metadata_path(path);
    let filemtime = input_file.metadata()?.modified()?;

    if !options.skip_checksum && metadata_path.exists() {
        let metadata = Metadata::read_from_path(&metadata_path)
            .with_context(|| format!("Failed to read metadata: {:?}", metadata_path))?;

        if validate_checksum(&recipients_data, &metadata.checksum.recipients, &path).is_ok()
            && validate_checksum(input.as_slice(), &metadata.checksum.decrypted, &path).is_ok()
        {
            if !options.skip_timestamps {
                set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
            }
            return Ok(None);
        }
    }

    let output = {
        let mut reader = BufReader::new(input.as_slice());
        let mut buffer = vec![];
        let mut enc_writer =
            encryptor.wrap_output(ArmoredWriter::wrap_output(&mut buffer, format)?)?;
        std::io::copy(&mut reader, &mut enc_writer)?;
        enc_writer.finish().and_then(|armor| armor.finish())?;
        buffer.flush()?;
        buffer
    };

    let timestamp = DateTime::<Utc>::from(filemtime).to_rfc3339();
    let secret = SecretMetadata { timestamp };
    let checksum = {
        let encrypted = make_checksum(output.as_slice());
        let decrypted = make_checksum(input.as_slice());
        ChecksumMetadata {
            encrypted,
            decrypted,
            recipients: make_checksum(&recipients_data),
        }
    };
    let old_metadata = if !options.skip_preview && output_path.exists() && metadata_path.exists() {
        Metadata::read_from_path(&metadata_path).ok()
    } else {
        None
    };

    let (old_content, old_preview) = if let Some(old_metadata) = &old_metadata {
        let old_preview = old_metadata.preview.as_ref().map(|p| p.preview.as_str());

        let decrypt_options = DecryptOptions {
            mode: match &options.mode {
                EncryptionMode::Passphrase(pass) => DecryptionMode::Passphrase(pass.clone()),
                EncryptionMode::Recipients(_) => DecryptionMode::Identities(options.identities),
            },
            skip_gitignore: true,
            skip_timestamps: true,
            skip_checksum_encrypted: true,
            skip_checksum_decrypted: true,
        };

        let old_content = File::open(&output_path)
            .ok()
            .and_then(|f| decrypt_into_memory(f, &decrypt_options).ok());

        (old_content, old_preview)
    } else {
        (None, None)
    };

    let preview = if !options.skip_preview {
        generate_preview(
            path,
            &input,
            old_content.as_deref(),
            old_preview,
            &secret.timestamp,
        )
    } else {
        None
    };

    let metadata = Metadata {
        secret,
        checksum,
        preview,
    };

    // Write starts here ------------------------

    // First add to .gitignore before creating the encrypted file, because, why not!
    let gitignore = if !options.skip_gitignore {
        append_to_gitignore_if_absent(&path)?
    } else {
        None
    };

    std::fs::write(&output_path, output)
        .with_context(|| format!("Failed to write encrypted file: {:?}", output_path))?;

    if !options.skip_timestamps {
        set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
    }
    std::fs::write(&metadata_path, toml::to_string(&metadata)?)
        .with_context(|| format!("Failed to write metadata file: {:?}", metadata_path))?;

    Ok(Some(OperationResult {
        kind: OperationKind::Encrypt,
        input: path.to_path_buf(),
        output: output_path,
        gitignore: gitignore,
        metadata: Some(metadata_path),
    }))
}

pub fn encrypt_dir<'a>(
    path: &'a Path,
    options: &EncryptOptions,
) -> impl Iterator<Item = Result<OperationResult>> {
    walkdir::WalkDir::new(path)
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
    } else {
        Box::new(encrypt_file(path, options).transpose().into_iter())
    }
}
