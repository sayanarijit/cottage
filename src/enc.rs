use age::armor::ArmoredWriter;
use age::secrecy::SecretString;
use anyhow::{Context, Result, anyhow};
use filetime::{FileTime, set_file_mtime};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::{
    is_encrypted_path, project::append_to_gitignore_if_absent, to_decrypted_path, to_encrypted_path,
};

#[derive(Clone)]
pub enum EncryptionMode<'a> {
    Passphrase(String),
    Recipients(&'a [Box<dyn age::Recipient + Send>]),
}

#[derive(Clone)]
pub struct EncryptOptions<'a> {
    pub mode: EncryptionMode<'a>,
    pub armor: bool,
    pub skip_gitignore: bool,
    pub skip_timestamps: bool,
}

pub fn encrypt_file<'a>(
    path: &'a Path,
    options: &EncryptOptions,
) -> Result<(&'a Path, PathBuf, Option<PathBuf>)> {
    let output_path = to_encrypted_path(path);

    // First add to .gitignore before creating the encrypted file, because, why not!
    let gitignorefile = if !options.skip_gitignore {
        append_to_gitignore_if_absent(&output_path)?
    } else {
        None
    };

    let encryptor = match &options.mode {
        EncryptionMode::Passphrase(pass) => {
            age::Encryptor::with_user_passphrase(SecretString::from(pass.as_str()))
        }
        EncryptionMode::Recipients(recipients) => {
            age::Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref() as _))
                .map_err(|_| anyhow!("At least one recipient must be provided"))?
        }
    };

    let format = if options.armor {
        age::armor::Format::AsciiArmor
    } else {
        age::armor::Format::Binary
    };

    let input_file =
        File::open(path).with_context(|| format!("Failed to open input file: {:?}", path))?;
    let input_metadata = input_file.metadata()?;
    {
        let mut output_file = File::create(&output_path)
            .with_context(|| format!("Failed to create output file: {:?}", &output_path))?;
        let mut output = BufWriter::new(&mut output_file);
        let mut writer = encryptor.wrap_output(ArmoredWriter::wrap_output(&mut output, format)?)?;

        let mut reader = BufReader::new(input_file);
        std::io::copy(&mut reader, &mut writer)?;
        writer.finish().and_then(|armor| armor.finish())?;
    }

    if !options.skip_timestamps {
        set_file_mtime(
            &output_path,
            FileTime::from_system_time(input_metadata.modified()?),
        )?;
    }

    Ok((path, output_path, gitignorefile))
}

pub fn encrypt_dir<'a>(
    path: &'a Path,
    options: &EncryptOptions,
) -> impl Iterator<Item = Result<(PathBuf, PathBuf, Option<PathBuf>)>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_encrypted_path(e.path()))
        .filter_map(|e| to_decrypted_path(e.path()))
        .map(|path| {
            encrypt_file(&path, options)
                .map(|(input, output, gi)| (input.to_path_buf(), output, gi))
        })
}
