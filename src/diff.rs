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

                let rel_input = proj.relative_to_cwd(&op.input);
                let rel_output = proj.relative_to_cwd(&op.output);

                let old_label = format!("a/{}", rel_input.display());
                let new_label = format!("b/{}", rel_output.display());

                let encrypted_str = String::from_utf8_lossy(&encrypted_content);
                let decrypted_str = String::from_utf8_lossy(&decrypted_content);

                let (old_str, new_str) = match op.kind {
                    OperationKind::Decrypt => (decrypted_str, encrypted_str),
                    OperationKind::Encrypt => (encrypted_str, decrypted_str),
                };

                let diff = TextDiff::from_lines(&old_str, &new_str);

                // println!("diff --cottage {} {}", old_label, new_label);
                println!("--- {}", old_label);
                println!("+++ {}", new_label);

                for change in diff.iter_all_changes() {
                    let (sign, style) = match change.tag() {
                        ChangeTag::Delete => ("-", Style::new().red()),
                        ChangeTag::Insert => ("+", Style::new().green()),
                        ChangeTag::Equal => (" ", Style::new()),
                    };
                    print!("{}{}", style.apply_to(sign), style.apply_to(change));
                }
            }
        }
    }

    Ok(has_diff)
}
