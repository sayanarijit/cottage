pub(crate) mod clean;
pub mod cli;
pub(crate) mod dec;
pub(crate) mod diff;
pub(crate) mod enc;
pub(crate) mod identity;
pub(crate) mod metadata;
pub(crate) mod operation;
pub(crate) mod preview;
pub(crate) mod project;
pub(crate) mod recipients;
pub(crate) mod status;
pub(crate) mod sync;

pub(crate) use clean::{CleanOptions, clean_path};
pub(crate) use dec::{DecryptOptions, DecryptionMode, decrypt_into_memory, decrypt_path};
pub(crate) use diff::{DiffOptions, diff};
pub(crate) use enc::{EncryptOptions, EncryptionMode, encrypt_path};
pub(crate) use identity::{Identity, load_identities};
pub(crate) use metadata::{
    ChecksumMetadata, Metadata, PreviewFormat, PreviewMetadata, SecretMetadata, make_checksum,
    verify_checksum,
};
pub(crate) use operation::{
    Operation, OperationKind, OperationResult, is_encrypted_path, is_metadata_path,
    to_decrypted_path, to_encrypted_path, to_metadata_path,
};
pub(crate) use preview::generate_preview;
pub(crate) use project::{Project, remove_from_gitignore_if_present};

#[cfg(test)]
pub(crate) use project::{append_line_if_absent, append_to_gitignore_if_absent, get_root};
pub(crate) use recipients::{RecipientData, load_recipients};
pub(crate) use status::{StatusOptions, status_path};
pub(crate) use sync::{SyncOptions, sync_path};

#[cfg(test)]
mod tests;
