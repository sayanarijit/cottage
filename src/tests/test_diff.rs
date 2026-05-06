use crate::{EncryptOptions, Project, decrypt_into_memory, encrypt_path, status_path};
use age::secrecy::ExposeSecret;
use anyhow::Result;
use std::fs;

#[test]
fn test_diff_logic() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    std::env::set_current_dir(&dir)?;
    fs::create_dir(dir.path().join(".cottage"))?;

    let _proj = Project::load()?;
    let secret_path = dir.path().join("secret.txt");
    fs::write(&secret_path, "original content\n")?;

    let sk = age::x25519::Identity::generate();
    let pk = sk.to_public();

    let identity = crate::Identity::X25519(sk);
    let recipient = crate::RecipientData {
        recipient: crate::recipients::Recipient::X25519(pk.clone()),
        raw: pk.to_string().into_bytes(),
        path: None,
    };

    let options = EncryptOptions {
        recipients: vec![recipient.clone()],
        identities: vec![identity.clone()],
        armor: true,
        skip_gitignore: true,
        skip_timestamps: false,
        skip_preview: true,
        force: false,
        identity_path: _proj.identity_path().to_path_buf(),
        dry_run: false,
    };

    for res in encrypt_path(&secret_path, &options) {
        res?;
    }

    let encrypted_path = dir.path().join("secret.txt.cott.age");
    assert!(encrypted_path.exists());

    // Ensure mtime will be different
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Modify decrypted file
    fs::write(&secret_path, "modified content\n")?;

    // Check status
    let operations: Vec<_> =
        status_path(&secret_path, crate::StatusOptions::default()).collect::<Result<Vec<_>>>()?;
    assert_eq!(operations.len(), 1);

    // Simulate diff logic
    let decrypted_content = fs::read(&secret_path)?;
    let encrypted_file = fs::File::open(&encrypted_path)?;

    let decrypt_options = crate::DecryptOptions {
        identities: vec![identity.clone()],
        recipients: vec![recipient.clone()],
        skip_gitignore: true,
        skip_timestamps: true,
        skip_verify_encrypted: false,
        skip_verify_recipients: false,
        dry_run: false,
    };

    let decrypted_from_encrypted = decrypt_into_memory(encrypted_file, &decrypt_options)?;

    assert_ne!(decrypted_content, decrypted_from_encrypted.expose_secret());
    assert_eq!(
        decrypted_from_encrypted.expose_secret(),
        b"original content\n"
    );
    assert_eq!(decrypted_content, b"modified content\n");

    Ok(())
}
