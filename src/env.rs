use crate::{DecryptOptions, Project, decrypt_into_memory, is_encrypted_path};
use age::secrecy::ExposeSecret;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub struct EnvOptions {
    pub command: String,
    pub args: Vec<String>,
    pub file: Option<PathBuf>,
    pub decrypt_options: DecryptOptions,
    pub dry_run: bool,
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

    let infile = File::open(&envfile).with_context(|| {
        format!(
            "{}: failed to open env file",
            proj.relative_to_cwd(&envfile).display()
        )
    })?;
    let reader = std::io::BufReader::new(infile);
    let secret = decrypt_into_memory(reader, &opts.decrypt_options)?;

    let res = if opts.dry_run {
        log::info!("dry run: skipping running the command");
        Ok((true, Some(0)))
    } else {
        if dotenvy::from_read(BufReader::new(secret.expose_secret())).is_err() {
            unsafe {
                // Safe because cottage is single threaded.
                std::env::set_var(
                "COTTAGE_SECRET",
                String::from_utf8(secret.expose_secret().to_vec()).with_context(|| {
                    format!(
                        "{}: secret is not valid UTF-8 and cannot be exported as COTTAGE_SECRET",
                        proj.relative_to_cwd(&envfile).display()
                    )
                })?,
            );
            }
        };
        log::info!(
            "running command: {:?} with args: {:?}",
            &opts.command,
            &opts.args
        );
        std::process::Command::new(&opts.command)
            .args(&opts.args)
            .status()
            .map(|s| (s.success(), s.code()))
    };

    let (is_success, status_code) = res?;
    if !is_success {
        std::process::exit(status_code.unwrap_or(1));
    }

    Ok(())
}
