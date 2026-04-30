use crate::{
    is_encrypted_path,
    project::{OperationResult, append_to_gitignore_if_absent},
    to_decrypted_path,
};
use age::armor::ArmoredReader;
use age::secrecy::SecretString;
use anyhow::{Context, Result, anyhow};
use filetime::{FileTime, set_file_mtime};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::iter;
use std::path::Path;

#[derive(Clone)]
pub enum DecryptionMode<'a> {
    Passphrase(String),
    Identities(&'a [Box<dyn age::Identity>]),
}

#[derive(Clone)]
pub struct DecryptOptions<'a> {
    pub mode: DecryptionMode<'a>,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
}

pub fn decrypt_file<'a>(path: &'a Path, options: &DecryptOptions) -> Result<OperationResult> {
    if !is_encrypted_path(path) {
        return Err(anyhow!(
            "Input file does not have .cott.age extension: {:?}",
            path
        ));
    }

    let output_path = to_decrypted_path(path).ok_or_else(|| {
        anyhow!(
            "Failed to determine decrypted path for input file: {:?}",
            path
        )
    })?;

    // First add to .gitignore before creating the decrypted file, so that if the operation fails
    // for some reason, we won't have a decrypted file that is not ignored.
    let gitignorefile = if !options.skip_gitignore {
        append_to_gitignore_if_absent(&output_path)?
    } else {
        None
    };

    let input_file =
        File::open(path).with_context(|| format!("Failed to open input file: {:?}", path))?;

    let input_metadata = input_file.metadata()?;
    {
        let input = BufReader::new(input_file);
        let decryptor = age::Decryptor::new_buffered(ArmoredReader::new(input))?;

        let output_file = File::create(&output_path)
            .with_context(|| format!("Failed to create output file: {:?}", &output_path))?;

        let mut decrypted = match &options.mode {
            DecryptionMode::Passphrase(pass) => decryptor.decrypt(iter::once(
                &age::scrypt::Identity::new(SecretString::from(pass.as_str())) as _,
            ))?,
            DecryptionMode::Identities(identities) => {
                decryptor.decrypt(identities.iter().map(|i| i.as_ref()))?
            }
        };

        let mut writer = BufWriter::new(output_file);
        std::io::copy(&mut decrypted, &mut writer)?;
        writer.flush()?;
    }

    if !options.skip_timestamps {
        set_file_mtime(
            &output_path,
            FileTime::from_system_time(input_metadata.modified()?),
        )?;
    };

    Ok(OperationResult {
        input: path.to_path_buf(),
        output: output_path,
        gitignore: gitignorefile,
    })
}

pub fn decrypt_dir<'a>(
    path: &'a Path,
    options: &DecryptOptions,
) -> impl Iterator<Item = Result<OperationResult>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_encrypted_path(e.path()))
        .map(|e| decrypt_file(e.path(), options))
}

pub fn decrypt_path<'a>(
    path: &'a Path,
    options: &'a DecryptOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    if path.is_dir() {
        Box::new(decrypt_dir(path, options))
    } else {
        Box::new(iter::once(decrypt_file(path, options)))
    }
}
