pub(crate) mod cat;
pub(crate) mod clean;
pub mod cli;
pub(crate) mod dec;
pub(crate) mod diff;
pub(crate) mod edit;
pub(crate) mod enc;
pub(crate) mod env;
pub(crate) mod identity;
pub(crate) mod metadata;
pub(crate) mod operation;
pub(crate) mod prefix;
pub(crate) mod preview;
pub(crate) mod project;
pub(crate) mod pull;
pub(crate) mod push;
pub(crate) mod recipients;
pub(crate) mod run;
pub(crate) mod status;
pub(crate) mod sync;
pub(crate) mod tee;
pub(crate) mod verify;

pub(crate) use cat::{CatOptions, cat};
pub(crate) use clean::{CleanOptions, clean_path};
pub(crate) use dec::{DecryptOptions, decrypt_file, decrypt_into_memory, decrypt_path};
pub(crate) use diff::{DiffOptions, diff};
pub(crate) use edit::{EditOptions, edit};
pub(crate) use enc::{EncryptOptions, encrypt_file, encrypt_path};
pub(crate) use env::{EnvOptions, decrypt_into_cmd, env};
pub(crate) use identity::{Identity, load_identities};
pub(crate) use metadata::{
    ChecksumMetadata, Metadata, PreviewFormat, PreviewMetadata, SecretMetadata, UpstreamMetadata,
    make_checksum, verify_checksum,
};
pub(crate) use operation::{
    Operation, OperationKind, OperationResult, TempDecryptedFile, clean_decrypted_secrets,
    decrypt_required_secrets, print_result, run_upstream_script, secure_remove_file,
};
pub(crate) use prefix::{
    PathConfig, PathPrefix, is_encrypted_path, is_metadata_path, prefix_revert, prefix_sync,
};
pub(crate) use preview::generate_preview;
pub(crate) use project::{Project, remove_from_gitignore_if_present};
pub(crate) use pull::{PullOptions, pull_path};
pub(crate) use push::{PushOptions, push_path};

pub(crate) use recipients::{
    RecipientData, filter_recipients_by_metadata, load_recipients, make_recipients_checksum_data,
};
pub(crate) use run::{RunOptions, run};
pub(crate) use status::{StatusOptions, status_path};
pub(crate) use sync::{SyncOptions, sync_path};
pub(crate) use tee::tee;
pub(crate) use verify::{VerifyOptions, verify_file, verify_path};

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use project::{append_line_if_absent, append_to_gitignore_if_absent, get_root};
