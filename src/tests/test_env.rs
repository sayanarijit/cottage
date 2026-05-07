use crate::{DecryptOptions, Project, decrypt_into_memory};
use age::secrecy::ExposeSecret;
use assert_fs::prelude::*;
use std::fs::File;

#[test]
fn test_decrypt_into_memory() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    proj_dir.create_dir_all().unwrap();

    let proj = Project::generate_test_project(&proj_dir);
    proj.init_test_recipients();

    let input_path = proj_dir.join("secret");
    let secret_path = crate::to_encrypted_path(&input_path);
    let secret_content = "key=value\nFOO=BAR";

    // Encrypt a file manually or using project methods if available
    // For simplicity in unit test, we'll use the project's encryption capability if possible
    // but here we just want to test decrypt_into_memory specifically.

    let identities = proj.load_test_identities().collect();
    let recipients = proj.load_test_recipients().collect();

    let enc_options = crate::EncryptOptions {
        recipients: recipients,
        identities: identities,
        armor: true,
        skip_gitignore: true,
        skip_timestamps: true,
        skip_preview: true,
        force: true,
        identity_path: proj.identity_path().to_path_buf(),
        dry_run: false,
    };

    let input_path = proj_dir.join("secret");
    std::fs::write(&input_path, secret_content).unwrap();
    crate::encrypt_file(&input_path, &enc_options, None).unwrap();

    let identities = proj.load_test_identities().collect();
    let recipients = proj.load_test_recipients().collect();
    let dec_options = DecryptOptions {
        identities,
        recipients,
        skip_gitignore: true,
        skip_timestamps: true,
        skip_verify_encrypted: false,
        skip_verify_recipients: false,
        dry_run: false,
    };

    let encrypted_file = File::open(&secret_path).unwrap();
    let decrypted = decrypt_into_memory(encrypted_file, &dec_options).unwrap();

    assert_eq!(decrypted.expose_secret(), secret_content.as_bytes());
}
