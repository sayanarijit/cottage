use cottage::*;
use std::fs;

#[test]
fn test_clean() {
    let dir = assert_fs::TempDir::new().unwrap();
    let proj_dir = dir.path();

    // Initialize project
    std::env::set_current_dir(proj_dir).unwrap();
    std::fs::create_dir_all(proj_dir.join(".cottage")).unwrap(); // Ensure

    let _proj = Project::load().unwrap();

    let secret_path = proj_dir.join("secret.txt");
    fs::write(&secret_path, "top secret").unwrap();

    let encrypted_path = to_encrypted_path(&secret_path);
    let metadata_path = to_metadata_path(&secret_path);

    fs::write(&encrypted_path, "encrypted content").unwrap();
    fs::write(&metadata_path, "metadata content").unwrap();

    let identity_path = proj_dir.join(".cottage/identity");
    assert!(identity_path.exists());
    assert!(encrypted_path.exists());
    assert!(metadata_path.exists());
    assert!(secret_path.exists());

    // Now we want to run clean.
    let opts = CleanOptions {
        dry_run: true,
        gitignore: false,
    };
    let cleaned = clean_path(&_proj.root(), &opts)
        .filter_map(|res| res.ok())
        .collect::<Vec<_>>();

    assert!(cleaned.len() == 1);
    assert!(cleaned.contains(&secret_path));
    assert!(!cleaned.contains(&identity_path));
    assert!(!cleaned.contains(&encrypted_path));
    assert!(!cleaned.contains(&metadata_path));
}
