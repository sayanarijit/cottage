use crate::{
    DecryptOptions, OperationResult, StatusOptions, TempDecryptedFile, decrypt_path,
    is_encrypted_path, status_path, to_decrypted_path, to_encrypted_path,
};
use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

pub struct RunOptions {
    pub command: String,
    pub args: Vec<String>,
    pub decrypt_options: DecryptOptions,
    pub dry_run: bool,
}

pub struct RunResult {
    pub operation_results: Box<dyn Iterator<Item = Result<OperationResult>>>,
    pub exit_code: i32,
}

pub fn run(
    proj_root: &Path,
    relative_cwd: impl Fn(&Path) -> PathBuf,
    opts: RunOptions,
) -> Result<RunResult> {
    let mut input_paths = vec![];
    let mut modified_args = vec![];
    for arg in opts.args.iter() {
        let p = PathBuf::from(arg);
        if is_encrypted_path(&p) && p.exists() {
            if let Some(dec) = to_decrypted_path(&p) {
                modified_args.push(dec.to_string_lossy().to_string());
            } else {
                modified_args.push(arg.clone());
            }
            input_paths.push(p);
        } else if to_encrypted_path(&p).exists() {
            modified_args.push(arg.clone());
            input_paths.push(to_encrypted_path(&p));
        } else if p.is_dir() {
            modified_args.push(arg.clone());
            input_paths.push(p);
        } else {
            modified_args.push(arg.clone());
        }
    }

    log::debug!("original args: {:?}", opts.args,);
    log::debug!("modified args: {:?}", modified_args);
    log::debug!("input paths: {:?}", input_paths);

    let input = if input_paths.is_empty() {
        vec![proj_root.into()]
    } else {
        input_paths
    };

    let status_opts = StatusOptions {
        skip_encryption: false,
        skip_decryption: true,
    };
    for path in input.iter() {
        if let Some(res) = status_path(path, status_opts).next() {
            let res = res?;
            anyhow::bail!(
                "{}: {} is dirty, please run `ctg sync` or `ctg encrypt` first",
                "pending encryption".red(),
                relative_cwd(&res.input).display()
            );
        }
    }

    let mut temp_files = vec![];
    for path in &input {
        if path.is_file() && is_encrypted_path(path) {
            if let Some(dec) = to_decrypted_path(path) {
                temp_files.push(TempDecryptedFile::new(dec, false));
            }
        } else if path.is_dir() {
            for entry in crate::iter_encrypted(path) {
                if let Some(dec) = to_decrypted_path(entry.path()) {
                    temp_files.push(TempDecryptedFile::new(dec, false));
                }
            }
        }
    }

    let mut results: Vec<Result<OperationResult>> = vec![];

    for path in &input {
        for res in decrypt_path(path, &opts.decrypt_options) {
            results.push(res);
        }
    }

    let res = if opts.dry_run {
        log::info!("dry run: skipping running the command");
        Ok((true, Some(0)))
    } else {
        log::info!(
            "running command: {:?} with args: {:?}",
            &opts.command,
            &modified_args
        );
        std::process::Command::new(&opts.command)
            .args(&modified_args)
            .status()
            .map(|s| (s.success(), s.code()))
    };

    for mut temp_file in temp_files {
        match temp_file.cleanup(opts.dry_run) {
            Ok(Some(res)) => results.push(Ok(res)),
            Ok(None) => {}
            Err(e) => results.push(Err(e)),
        }
    }

    let (is_success, status_code) = res?;
    let exit_code = if is_success {
        0
    } else {
        status_code.unwrap_or(1)
    };

    Ok(RunResult {
        operation_results: Box::new(results.into_iter()),
        exit_code,
    })
}
