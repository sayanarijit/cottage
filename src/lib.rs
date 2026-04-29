mod dec;
mod enc;
mod identity;
mod project;
mod recipients;
mod sync;

pub mod cli;
pub use dec::{DecryptOptions, DecryptionMode, decrypt_dir, decrypt_file};
pub use enc::{EncryptOptions, EncryptionMode, encrypt_dir, encrypt_file};
pub use identity::{load_identities, parse_identities_dir, parse_identity_file};
pub use project::{
    get_project_root, get_root, is_encrypted_path, to_decrypted_path, to_encrypted_path,
};
pub use recipients::{
    load_recipients, parse_recipient, parse_recipients_dir, parse_recipients_file,
};
pub use sync::{sync_dir, sync_file};
