use crate::Project;
use anyhow::Result;
use colored::Colorize;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Encrypt,
    Decrypt,
    Delete,
}

impl std::fmt::Display for OperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationKind::Encrypt => write!(f, "{}", "encrypt".green()),
            OperationKind::Decrypt => write!(f, "{}", "decrypt".cyan()),
            OperationKind::Delete => write!(f, "{}", "delete ".red()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub kind: OperationKind,
    pub input: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationResult {
    pub kind: OperationKind,
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub edits: Vec<PathBuf>,
    pub cleanups: Vec<PathBuf>,
}

impl From<Operation> for OperationResult {
    fn from(op: Operation) -> Self {
        Self {
            kind: op.kind,
            input: op.input,
            output: Some(op.output),
            edits: vec![],
            cleanups: vec![],
        }
    }
}

pub fn print_result(
    mut file: impl Write,
    proj: &Project,
    op: &OperationResult,
    compact: bool,
) -> Result<()> {
    match (op.kind, compact, op.output.as_ref()) {
        (kind, false, Some(output)) => {
            writeln!(
                file,
                "{} {}\n   {} {}",
                kind,
                proj.relative_to_cwd(&op.input).display(),
                "into".blue(),
                proj.relative_to_cwd(output).display()
            )?;
            print_cleanups(&mut file, proj, &op.cleanups)?;
            print_edits(&mut file, proj, &op.edits)?;
        }
        (kind, false, None) => {
            writeln!(
                file,
                "{} {}",
                kind,
                proj.relative_to_cwd(&op.input).display()
            )?;
            print_cleanups(&mut file, proj, &op.cleanups)?;
            print_edits(&mut file, proj, &op.edits)?;
        }
        (_, true, _) => {
            writeln!(
                file,
                "{}",
                proj.relative_to_cwd(&op.input)
                    .display()
                    .to_string()
                    .green()
            )?;
        }
    }
    Ok(())
}

fn print_edits(mut file: impl Write, proj: &Project, edits: &[PathBuf]) -> Result<()> {
    for path in edits {
        writeln!(
            file,
            "   {} {}",
            "edit".yellow(),
            proj.relative_to_cwd(&path).display()
        )?;
    }
    Ok(())
}

fn print_cleanups(mut file: impl Write, proj: &Project, cleanups: &[PathBuf]) -> Result<()> {
    for path in cleanups {
        writeln!(
            file,
            "{} {}",
            " delete".red(),
            proj.relative_to_cwd(&path).display()
        )?;
    }
    Ok(())
}

pub fn to_encrypted_path(path: &Path) -> PathBuf {
    path.with_added_extension("cott")
        .with_added_extension("age")
}

pub fn to_metadata_path(path: &Path) -> PathBuf {
    path.with_added_extension("cott")
        .with_added_extension("toml")
}

pub fn is_encrypted_path(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".cott.age")
}

pub fn is_metadata_path(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".cott.toml")
}

pub fn to_decrypted_path(path: &Path) -> Option<PathBuf> {
    if is_encrypted_path(path) {
        path.file_stem()
            .and_then(|s| PathBuf::from(s).file_stem().map(|s| path.with_file_name(s)))
    } else {
        None
    }
}
