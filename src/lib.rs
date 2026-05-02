mod clean;
pub mod cli;
mod dec;
mod diff;
mod enc;
mod identity;
mod metadata;
mod operation;
mod preview;
mod project;
mod recipients;
mod sync;

pub use clean::{CleanOptions, clean_dir, clean_path, clean_project};
pub use dec::{DecryptOptions, DecryptionMode, decrypt_into_memory, decrypt_path};
pub use diff::{DiffOptions, diff};
pub use enc::{EncryptOptions, EncryptionMode, encrypt_path};
pub use identity::{Identity, load_identities};
pub use metadata::{
    ChecksumMetadata, Metadata, PreviewFormat, PreviewMetadata, SecretMetadata, make_checksum,
    verify_checksum,
};
pub use operation::{
    Operation, OperationKind, OperationResult, is_encrypted_path, is_metadata_path,
    to_decrypted_path, to_encrypted_path, to_metadata_path,
};
pub use preview::generate_preview;
pub use project::{
    Project, append_line_if_absent, append_to_gitignore_if_absent, get_project_root, get_root,
    remove_from_gitignore_if_present, remove_line_if_present,
};
pub use recipients::{Recipient, RecipientData, load_recipients};
pub use sync::{SyncOptions, status_path, sync_path};
