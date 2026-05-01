use crate::{
    DecryptOptions, DecryptionMode, OperationKind, Project, decrypt_into_memory, status_path,
};
use anyhow::Result;
use console::Style;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::PathBuf;

pub struct DiffOptions<'a> {
    pub mode: DecryptionMode<'a>,
    pub skip_checksum_encrypted: bool,
    pub skip_checksum_decrypted: bool,
}

pub fn diff(proj: &Project, paths: &[PathBuf], options: &DiffOptions) -> Result<bool> {
    let mut has_diff = false;

    let decrypt_options = DecryptOptions {
        mode: options.mode.clone(),
        skip_gitignore: true,
        skip_timestamps: true,
        skip_checksum_encrypted: options.skip_checksum_encrypted,
        skip_checksum_decrypted: options.skip_checksum_decrypted,
    };

    for path in paths {
        for res in status_path(path) {
            let op = res?;
            let (decrypted_path, encrypted_path) = match op.kind {
                OperationKind::Encrypt => (&op.input, &op.output),
                OperationKind::Decrypt => (&op.output, &op.input),
            };

            let decrypted_content = if decrypted_path.exists() {
                fs::read(&decrypted_path)?
            } else {
                vec![]
            };

            let encrypted_content = if encrypted_path.exists() {
                let file = fs::File::open(&encrypted_path)?;
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

                let diff_path = proj.relative_to_root(&decrypted_path);
                let styled_diff_path = Style::new().cyan().apply_to(diff_path.display());

                println!(
                    "{}",
                    Style::new().dim().apply_to(format!(
                        "diff --git a/{} b/{}",
                        diff_path.display(),
                        diff_path.display()
                    )),
                );
                println!(
                    "{} a/{styled_diff_path}\n{} b/{styled_diff_path}",
                    Style::new().red().apply_to("---"),
                    Style::new().green().apply_to("+++"),
                );

                for change in diff.iter_all_changes() {
                    let sign = match change.tag() {
                        ChangeTag::Delete => Style::new().red().apply_to("-"),
                        ChangeTag::Insert => Style::new().green().apply_to("+"),
                        ChangeTag::Equal => Style::new().dim().apply_to(" "),
                    };
                    print!("{}{}", sign, change);
                }
            }
        }
    }

    Ok(has_diff)
}
