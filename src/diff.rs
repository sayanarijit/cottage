use crate::{
    DecryptOptions, Identity, OperationKind, Project, RecipientData, StatusOptions,
    decrypt_into_memory, status_path,
};
use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::Result;
use colored::Colorize;
use similar::TextDiff;
use std::fs;
use std::path::PathBuf;

pub struct DiffOptions {
    pub identities: Vec<Identity>,
    pub recipients: Vec<RecipientData>,
    pub skip_encryption: bool,
    pub skip_decryption: bool,
    pub skip_verify_encrypted: bool,
    pub skip_verify_recipients: bool,
}

pub fn diff(proj: &Project, paths: &[PathBuf], options: DiffOptions) -> Result<bool> {
    let mut has_diff = false;

    let status_opts = StatusOptions {
        skip_encryption: options.skip_encryption,
        skip_decryption: options.skip_decryption,
    };

    let dec_opts = DecryptOptions {
        identities: options.identities.clone(),
        recipients: options.recipients.clone(),
        skip_gitignore: true,
        skip_timestamps: true,
        skip_verify_encrypted: options.skip_verify_encrypted,
        skip_verify_recipients: options.skip_verify_recipients,
        dry_run: true,
    };

    for path in paths {
        for res in status_path(path, status_opts) {
            let op = res?;
            let (decrypted_path, encrypted_path) = match op.kind {
                OperationKind::Encrypt => (&op.input, &op.output),
                OperationKind::Decrypt => (&op.output, &op.input),
                OperationKind::Delete => {
                    unimplemented!("diff: delete operations are not supported");
                }
                OperationKind::Pull | OperationKind::Push => unreachable!(),
            };

            let decrypted_content = if decrypted_path.exists() {
                SecretSlice::from(fs::read(decrypted_path)?)
            } else {
                SecretSlice::new(vec![].into())
            };

            let encrypted_content = if encrypted_path.exists() {
                let file = fs::File::open(encrypted_path)?;
                decrypt_into_memory(file, &dec_opts)?
            } else {
                SecretSlice::new(vec![].into())
            };

            let encrypted_str = String::from_utf8_lossy(encrypted_content.expose_secret());
            let decrypted_str = String::from_utf8_lossy(decrypted_content.expose_secret());
            if encrypted_str != decrypted_str {
                has_diff = true;

                let (old_str, new_str) = match op.kind {
                    OperationKind::Encrypt => (encrypted_str, decrypted_str),
                    OperationKind::Decrypt => (decrypted_str, encrypted_str),
                    OperationKind::Delete => {
                        unimplemented!("diff: delete operations are not supported");
                    }
                    OperationKind::Pull | OperationKind::Push => unreachable!(),
                };

                let diff = TextDiff::from_lines(&old_str, &new_str);

                let diff_path = proj.relative_to_root(decrypted_path);
                let a_path = format!("a/{}", diff_path.display());
                let b_path = format!("b/{}", diff_path.display());

                println!(
                    "{}",
                    format!("diff --git {} {}", a_path, b_path).bright_black(),
                );

                let mut unified_diff = diff.unified_diff();
                let unified_diff = unified_diff.context_radius(3).header(&a_path, &b_path);

                for line in format!("{}", unified_diff).lines() {
                    if line.starts_with("@@") {
                        println!("{}", line.cyan());
                    } else if line.starts_with("---") {
                        println!("{}", line.red().bold());
                    } else if line.starts_with("+++") {
                        println!("{}", line.green().bold());
                    } else if line.starts_with('-') {
                        println!("{}", line.red());
                    } else if line.starts_with('+') {
                        println!("{}", line.green());
                    } else {
                        println!("{}", line);
                    }
                }
            }
        }
    }

    Ok(has_diff)
}
