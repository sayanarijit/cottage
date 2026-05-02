use crate::{
    DecryptOptions, DecryptionMode, OperationKind, Project, decrypt_into_memory, status_path,
};
use anyhow::Result;
use owo_colors::OwoColorize;
use similar::TextDiff;
use std::fs;
use std::path::PathBuf;

pub struct DiffOptions {
    pub mode: DecryptionMode,
    pub skip_verify_encrypted: bool,
    pub skip_checksum_decrypted: bool,
}

pub fn diff(proj: &Project, paths: &[PathBuf], options: DiffOptions) -> Result<bool> {
    let mut has_diff = false;

    let decrypt_options = DecryptOptions {
        mode: options.mode,
        skip_gitignore: true,
        skip_timestamps: true,
        skip_verify_encrypted: options.skip_verify_encrypted,
        skip_verify_decrypted: options.skip_checksum_decrypted,
    };

    for path in paths {
        for res in status_path(path) {
            let op = res?;
            let (decrypted_path, encrypted_path) = match op.kind {
                OperationKind::Encrypt => (&op.input, &op.output),
                OperationKind::Decrypt => (&op.output, &op.input),
            };

            let decrypted_content = if decrypted_path.exists() {
                fs::read(decrypted_path)?
            } else {
                vec![]
            };

            let encrypted_content = if encrypted_path.exists() {
                let file = fs::File::open(encrypted_path)?;
                decrypt_into_memory(file, &decrypt_options)?
            } else {
                vec![]
            };

            if decrypted_content != encrypted_content {
                has_diff = true;

                let encrypted_str = String::from_utf8_lossy(&encrypted_content);
                let decrypted_str = String::from_utf8_lossy(&decrypted_content);

                let (old_str, new_str) = match op.kind {
                    OperationKind::Encrypt => (encrypted_str, decrypted_str),
                    OperationKind::Decrypt => (decrypted_str, encrypted_str),
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
