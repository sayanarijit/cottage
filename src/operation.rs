use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum OperationKind {
    Encrypt,
    Decrypt,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub kind: OperationKind,
    pub input: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct OperationResult {
    pub kind: OperationKind,
    pub input: PathBuf,
    pub output: PathBuf,
    pub metadata: Option<PathBuf>,
    pub gitignore: Option<PathBuf>,
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
