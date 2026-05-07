use crate::*;
use std::fs;

#[test]
fn test_clean() {
    let dir = assert_fs::TempDir::new().unwrap();
    std::env::set_current_dir(&dir).unwrap();

    // Initialize project
    std::fs::create_dir_all(&dir.join(".cottage")).unwrap();
    std::fs::create_dir_all(&dir.join(".git")).unwrap();

    let secret_path = dir.join("secret.txt");
    fs::write(&secret_path, "top secret").unwrap();

    let encrypted_path = to_encrypted_path(&secret_path);
    let metadata_path = to_metadata_path(&secret_path);

    fs::write(&encrypted_path, "encrypted content").unwrap();
    fs::write(&metadata_path, "metadata content").unwrap();

    // Add to .gitignore
    append_to_gitignore_if_absent(&secret_path, false).unwrap();

    assert!(encrypted_path.exists());
    assert!(metadata_path.exists());
    assert!(secret_path.exists());

    // Dry run
    let opts = CleanOptions {
        dry_run: true,
        encrypted: true,
        gitignore: true,
    };
    let cleaned = clean_path(&dir, &opts)
        .filter_map(|res| res.ok())
        .collect::<Vec<_>>();

    assert!(cleaned.len() == 1);
    assert!(cleaned.iter().any(|r| r.input == secret_path));

    // Verify nothing was actually deleted
    assert!(secret_path.exists());
    assert!(encrypted_path.exists());
    assert!(metadata_path.exists());

    // Actual clean
    let opts = CleanOptions {
        dry_run: false,
        encrypted: true,
        gitignore: true,
    };
    let cleaned = clean_path(&dir, &opts)
        .filter_map(|res| res.ok())
        .collect::<Vec<_>>();

    assert!(cleaned.len() == 1);
    assert!(cleaned.iter().any(|r| r.input == secret_path));

    // Verify everything was deleted
    assert!(!secret_path.exists());
    assert!(!encrypted_path.exists());
    assert!(!metadata_path.exists());

    // Test cleaning when decrypted file is missing
    fs::write(&encrypted_path, "encrypted content").unwrap();
    let opts = CleanOptions {
        dry_run: false,
        encrypted: true,
        gitignore: false,
    };
    let cleaned = clean_path(&secret_path, &opts)
        .filter_map(|res| res.ok())
        .collect::<Vec<_>>();
    assert!(cleaned.iter().any(|r| r.input == secret_path));
    assert!(!encrypted_path.exists());
}
