use anyhow::Result;
use cottage::{
    EncryptOptions, EncryptionMode, Project, decrypt_into_memory, encrypt_path, status_path,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_diff_logic() -> Result<()> {
    let dir = tempdir()?;
    std::env::set_current_dir(&dir)?;
    fs::create_dir(dir.path().join(".cottage"))?;

    let _proj = Project::init()?;
    let secret_path = dir.path().join("secret.txt");
    fs::write(&secret_path, "original content\n")?;

    let options = EncryptOptions {
        mode: EncryptionMode::Passphrase("password".to_string()),
        identities: &[],
        armor: true,
        skip_gitignore: true,
        skip_timestamps: false,
        skip_checksum: false,
        skip_preview: true,
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
    let operations: Vec<_> = status_path(&secret_path).collect::<Result<Vec<_>>>()?;
    assert_eq!(operations.len(), 1);
    
    // Simulate diff logic
    let decrypted_content = fs::read(&secret_path)?;
    let encrypted_file = fs::File::open(&encrypted_path)?;
    
    let decrypt_options = cottage::DecryptOptions {
        mode: cottage::DecryptionMode::Passphrase("password".to_string()),
        skip_gitignore: true,
        skip_timestamps: true,
        skip_checksum_encrypted: false,
        skip_checksum_decrypted: false,
    };
    
    let decrypted_from_encrypted = decrypt_into_memory(encrypted_file, &decrypt_options)?;
    
    assert_ne!(decrypted_content, decrypted_from_encrypted);
    assert_eq!(decrypted_from_encrypted, b"original content\n");
    assert_eq!(decrypted_content, b"modified content\n");

    Ok(())
}
