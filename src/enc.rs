use age::armor::ArmoredWriter;
use age::secrecy::SecretString;
use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

pub enum EncryptionMode<'a> {
    Passphrase(String),
    Recipients(&'a [Box<dyn age::Recipient + Send>]),
}

pub struct EncryptOptions<'a> {
    pub mode: EncryptionMode<'a>,
    pub armor: bool,
}

pub fn encrypt_file<'a>(
    input_path: &'a Path,
    options: &EncryptOptions,
) -> Result<(&'a Path, PathBuf)> {
    let input_file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {:?}", input_path))?;

    let output_path = input_path
        .with_added_extension("cott")
        .with_added_extension("age");

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

    let mut output_file = File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {:?}", &output_path))?;
    let mut output = BufWriter::new(&mut output_file);
    let mut writer = encryptor.wrap_output(ArmoredWriter::wrap_output(&mut output, format)?)?;

    let mut reader = BufReader::new(input_file);
    std::io::copy(&mut reader, &mut writer)?;
    writer.finish().and_then(|armor| armor.finish())?;

    Ok((input_path, output_path))
}

pub fn encrypt_dir<'a>(
    path: &'a Path,
    options: &EncryptOptions,
) -> impl Iterator<Item = Result<(PathBuf, PathBuf)>> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().to_string_lossy().ends_with(".cott.age"))
        .filter_map(|e| {
            e.path().file_stem().and_then(|s| {
                PathBuf::from(s)
                    .file_stem()
                    .map(|s| e.path().with_file_name(s))
            })
        })
        .map(|path| {
            encrypt_file(&path, options).map(|(input, output)| (input.to_path_buf(), output))
        })
}
