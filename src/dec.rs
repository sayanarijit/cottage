use crate::{
    Identity, Metadata, OperationKind, OperationResult, is_encrypted_path,
    project::append_to_gitignore_if_absent, to_decrypted_path, to_metadata_path, verify_checksum,
};
use age::armor::ArmoredReader;
use age::secrecy::{ExposeSecret, SecretSlice, SecretString};
use anyhow::{Context, Result};
use filetime::{FileTime, set_file_mtime};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Clone, Debug)]
pub enum DecryptionMode {
    Passphrase(SecretString),
    Identities(Vec<Identity>),
}

pub struct DecryptOptions {
    pub mode: DecryptionMode,
    pub dry_run: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_verify_encrypted: bool,
    pub skip_verify_decrypted: bool,
}

pub fn decrypt_into_memory(
    input_reader: impl Read,
    opts: &DecryptOptions,
) -> Result<SecretSlice<u8>> {
    let decryptor = age::Decryptor::new_buffered(ArmoredReader::new(input_reader))?;
    let mut buffer = vec![];

    let mut decrypted = match &opts.mode {
        DecryptionMode::Passphrase(passphrase) => {
            let identity = age::scrypt::Identity::new(passphrase.clone());
            decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
        }
        DecryptionMode::Identities(identities) => {
            let age_identities: Vec<Box<dyn age::Identity>> =
                identities.iter().map(|id| id.clone().into()).collect();
            decryptor.decrypt(age_identities.iter().map(|id| id.as_ref()))
        }
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

    if !opts.skip_verify_encrypted {
        verify_checksum(&input, &metadata.checksum.encrypted, path)?;
    }

    let output = decrypt_into_memory(input_file, opts)?;

    if output_path.exists() && std::fs::read(&output_path)? == output.expose_secret().to_vec() {
        log::debug!("{}: skipping write: content matches", output_path.display());
        if !opts.skip_timestamps {
            set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
        };
        return Ok(None);
    }

    if !opts.skip_verify_decrypted {
        verify_checksum(&output, &metadata.checksum.decrypted, &output_path)?;
    }

    // Write starts here ------------------------

    // First add to .gitignore before creating the decrypted file, so that if the operation fails
    // for some reason, we won't have a decrypted file that is not ignored.
    let gitignorefile = if !opts.skip_gitignore {
        append_to_gitignore_if_absent(&output_path, opts.dry_run)?
    } else {
        None
    };

    if opts.dry_run {
        log::debug!(
            "{}: dry-run: skipping write of decrypted file",
            output_path.display()
        );
    } else {
        log::debug!("{}: writing decrypted file", output_path.display());
        std::fs::write(&output_path, output.expose_secret()).with_context(|| {
            format!("{}: could not write decrypted file", output_path.display())
        })?;
        if !opts.skip_timestamps {
            set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
        };
    };

    Ok(Some(OperationResult {
        kind: OperationKind::Decrypt,
        input: path.to_path_buf(),
        output: output_path,
        gitignore: gitignorefile,
        metadata: None,
    }))
}

pub fn decrypt_dir(
    path: &Path,
    options: &DecryptOptions,
) -> impl Iterator<Item = Result<OperationResult>> {
    walkdir::WalkDir::new(path)
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
