use crate::{DecryptOptions, Project, decrypt_into_memory, is_encrypted_path};
use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct EnvOptions {
    pub command: String,
    pub args: Vec<String>,
    pub file: Option<PathBuf>,
    pub decrypt_options: DecryptOptions,
    pub dry_run: bool,
}

pub fn set_all_vars(cmd: &mut Command, secret: &SecretSlice<u8>) -> Result<()> {
    let mut reader = BufReader::new(secret.expose_secret());
    for item in dotenvy::from_read_iter(&mut reader) {
        let (key, value) = item?;
        cmd.env(key, value);
    }
    Ok(())
}

pub fn decrypt_into_cmd(
    proj: &Project,
    cmd: &mut Command,
    envfile: &Path,
    decrypt_options: &DecryptOptions,
) -> Result<()> {
    let infile = File::open(envfile).with_context(|| {
        format!(
            "{}: failed to open env file",
            proj.relative_to_cwd(envfile).display()
        )
    })?;
    let reader = std::io::BufReader::new(infile);
    let secret = decrypt_into_memory(reader, decrypt_options)?;
    set_all_vars(cmd, &secret).or_else(|_| {
        let filename = envfile
            .file_name()
            .unwrap_or_default()
            .display()
            .to_string();

        if filename.starts_with(".env") {
            log::warn!(
                "{}: failed to parse env file as dotenv, falling back to setting COTTAGE_SECRET",
                proj.relative_to_cwd(envfile).display()
            );
        }
        cmd.env(
            "COTTAGE_SECRET",
            String::from_utf8_lossy(secret.expose_secret()).to_string(),
        );
        Ok(())
    })
}

pub fn env(proj: &Project, opts: EnvOptions) -> Result<()> {
    let envfile = if let Some(file) = opts.file {
        if is_encrypted_path(&file) {
            if file.exists() {
                file
            } else {
                anyhow::bail!(
                    "{}: specified env file does not exist",
                    proj.relative_to_cwd(&file).display()
                );
            }
        } else {
            anyhow::bail!(
                "{}: specified env file is not an encrypted file",
                proj.relative_to_cwd(&file).display()
            );
        }
    } else {
        proj.cwd().join(".env.cott.age")
    };

    if !envfile.exists() {
        anyhow::bail!(
            "{}: file does not exist, specify with --file or create one with `ctg edit .env`",
            proj.relative_to_cwd(&envfile).display()
        );
    }

    let res = if opts.dry_run {
        log::info!("dry run: skipping running the command");
        Ok((true, Some(0)))
    } else {
        log::info!(
            "running command: {:?} with args: {:?}",
            &opts.command,
            &opts.args
        );
        let mut cmd = std::process::Command::new(&opts.command);
        decrypt_into_cmd(proj, &mut cmd, &envfile, &opts.decrypt_options)?;
        cmd.args(opts.args)
            .status()
            .map(|s| (s.success(), s.code()))
    };

    let (is_success, status_code) = res?;
    if !is_success {
        std::process::exit(status_code.unwrap_or(1));
    }

    Ok(())
}
