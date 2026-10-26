use crate::{EncryptOptions, Project, encrypt_path, prefix_revert, prefix_sync};
use anyhow::Result;
use std::fs;

const PREFIX_CONFIG: &str =
    "[path.prefix]\nencrypted = \"./store/enc\"\nmetadata = \"./store/meta\"\n";

fn encrypt_options(proj: &Project) -> EncryptOptions {
    let sk = age::x25519::Identity::generate();
    let pk = sk.to_public();
    EncryptOptions {
        recipients: vec![crate::RecipientData {
            recipient: crate::recipients::Recipient::X25519(pk.clone()),
            raw: pk.to_string().into_bytes(),
            path: None,
        }],
        identities: vec![crate::Identity::X25519(sk)],
        armor: true,
        skip_gitignore: true,
        skip_timestamps: false,
        skip_preview: true,
        force: false,
        identity_path: proj.identity_path().to_path_buf(),
        dry_run: false,
    }
}

#[test]
fn test_prefixed_paths() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    let root = dir.path();
    let proj = Project::generate_test_project(root).with_toml_config(PREFIX_CONFIG)?;
    let prefix = proj.prefix();

    let secret = root.join("sub/a.txt");
    let encrypted = root.join("store/enc/sub/a.txt.cott.age");
    let metadata = root.join("store/meta/sub/a.txt.cott.toml");

    assert_eq!(prefix.to_encrypted_path(&secret), encrypted);
    assert_eq!(prefix.to_metadata_path(&secret), metadata);
    assert_eq!(prefix.to_decrypted_path(&encrypted), Some(secret.clone()));
    assert_eq!(prefix.to_decrypted_path(&metadata), Some(secret));

    // Directories are auto generated
    assert!(root.join("store/enc").is_dir());
    assert!(root.join("store/meta").is_dir());
    Ok(())
}

#[test]
fn test_encrypt_writes_to_prefix() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    let root = dir.path();
    let proj = Project::generate_test_project(root).with_toml_config(PREFIX_CONFIG)?;

    let secret = root.join("sub/a.txt");
    fs::create_dir_all(secret.parent().unwrap())?;
    fs::write(&secret, "hi\n")?;

    for res in encrypt_path(proj.prefix(), &secret, &encrypt_options(&proj)) {
        res?;
    }

    assert!(root.join("store/enc/sub/a.txt.cott.age").is_file());
    assert!(root.join("store/meta/sub/a.txt.cott.toml").is_file());
    assert!(!root.join("sub/a.txt.cott.age").exists());
    assert!(!root.join("sub/a.txt.cott.toml").exists());

    let found: Vec<_> = proj
        .prefix()
        .iter_encrypted(root)
        .map(|e| e.into_path())
        .collect();
    assert_eq!(found, vec![root.join("store/enc/sub/a.txt.cott.age")]);
    Ok(())
}

#[test]
fn test_prefix_sync_and_revert() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    let root = dir.path();
    let proj = Project::generate_test_project(root).with_toml_config(PREFIX_CONFIG)?;
    let prefix = proj.prefix();

    fs::create_dir_all(root.join("sub"))?;
    fs::write(root.join("sub/a.txt.cott.age"), "enc")?;
    fs::write(root.join("sub/a.txt.cott.toml"), "meta")?;

    let moves = prefix_sync(prefix, root, false)?;
    assert_eq!(moves.len(), 2);
    assert!(!root.join("sub/a.txt.cott.age").exists());
    assert!(!root.join("sub/a.txt.cott.toml").exists());
    assert_eq!(
        fs::read_to_string(root.join("store/enc/sub/a.txt.cott.age"))?,
        "enc"
    );
    assert_eq!(
        fs::read_to_string(root.join("store/meta/sub/a.txt.cott.toml"))?,
        "meta"
    );

    // Already prefixed: nothing left to move
    assert!(prefix_sync(prefix, root, false)?.is_empty());

    // Dry run moves nothing
    assert_eq!(prefix_revert(prefix, true)?.len(), 2);
    assert!(root.join("store/enc/sub/a.txt.cott.age").exists());

    assert_eq!(prefix_revert(prefix, false)?.len(), 2);
    assert_eq!(fs::read_to_string(root.join("sub/a.txt.cott.age"))?, "enc");
    assert_eq!(
        fs::read_to_string(root.join("sub/a.txt.cott.toml"))?,
        "meta"
    );
    assert!(!root.join("store/enc/sub").exists());
    Ok(())
}
