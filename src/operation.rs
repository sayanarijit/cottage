use std::path::PathBuf;

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
    pub gitignore: Option<PathBuf>,
}
