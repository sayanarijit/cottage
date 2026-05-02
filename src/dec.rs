use crate::{
    Identity, Metadata, OperationKind, OperationResult, is_encrypted_path,
    project::append_to_gitignore_if_absent, to_decrypted_path, to_metadata_path, verify_checksum,
};
use age::armor::ArmoredReader;
use age::secrecy::SecretString;
use anyhow::{Context, Result};
use filetime::{FileTime, set_file_mtime};
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom, Write};
use std::iter;
use std::path::Path;

#[derive(Clone)]
pub enum DecryptionMode {
    Passphrase(String),
    Identities(Vec<Identity>),
}

#[derive(Clone)]
pub struct DecryptOptions {
    pub mode: DecryptionMode,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
    pub skip_verify_encrypted: bool,
    pub skip_verify_decrypted: bool,
}

pub fn decrypt_into_memory(
    input_reader: impl std::io::Read,
    options: &DecryptOptions,
) -> Result<Vec<u8>> {
    let decryptor = age::Decryptor::new_buffered(ArmoredReader::new(input_reader))?;

    let mut buffer = vec![];

    let mut decrypted = match &options.mode {
        DecryptionMode::Passphrase(pass) => decryptor.decrypt(iter::once(
            &age::scrypt::Identity::new(SecretString::from(pass.as_str())) as _,
        ))?,
        DecryptionMode::Identities(identities) => {
            decryptor.decrypt(identities.iter().map(|id| id.as_ref() as _))?
        }
    };

    std::io::copy(&mut decrypted, &mut buffer)?;
    buffer.flush()?;
    Ok(buffer)
}

pub fn decrypt_file(path: &Path, options: &DecryptOptions) -> Result<Option<OperationResult>> {
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
        buffer
    };

    if !options.skip_verify_encrypted {
        verify_checksum(input.as_slice(), &metadata.checksum.encrypted, path)?;
    }

    let output = decrypt_into_memory(input_file, options)?;

    if output_path.exists() && std::fs::read(&output_path)? == output {
        log::debug!("{}: skipping write: content matches", output_path.display());
        if !options.skip_timestamps {
            set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
        };
        return Ok(None);
    }

    if !options.skip_verify_decrypted {
        verify_checksum(
            output.as_slice(),
            &metadata.checksum.decrypted,
            &output_path,
        )?;
    }

    // Write starts here ------------------------

    // First add to .gitignore before creating the decrypted file, so that if the operation fails
    // for some reason, we won't have a decrypted file that is not ignored.
    let gitignorefile = if !options.skip_gitignore {
        append_to_gitignore_if_absent(&output_path)?
    } else {
        None
    };

    log::debug!("{}: writing decrypted file", output_path.display());
    std::fs::write(&output_path, &output)
        .with_context(|| format!("{}: could not write decrypted file", output_path.display()))?;
    if !options.skip_timestamps {
        set_file_mtime(&output_path, FileTime::from_system_time(filemtime))?;
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
    } else {
        Box::new(decrypt_file(path, options).transpose().into_iter())
    }
}
