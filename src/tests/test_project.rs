use crate::*;
use std::path::PathBuf;

#[test]
fn test_get_root() {
    let root = assert_fs::TempDir::new().unwrap();
    let subdir = root.path().join("subdir");

    std::fs::create_dir_all(subdir.join(".cottage")).unwrap();

    assert_eq!(
        get_root(&subdir.join("foo/bar"), ".cottage/"),
        Some(subdir.clone())
    );
    assert_eq!(get_root(&subdir.join("foo/bar"), ".git/"), None);
}

#[test]
fn test_to_encrypted_path() {
    let path = PathBuf::from("secret.txt");
    let encrypted_path = to_encrypted_path(&path);
    assert_eq!(encrypted_path, PathBuf::from("secret.txt.cott.age"));

    let dotenvpath = PathBuf::from(".env");
    let encrypted_dotenv_path = to_encrypted_path(&dotenvpath);
    assert_eq!(encrypted_dotenv_path, PathBuf::from(".env.cott.age"));
}

#[test]
fn test_is_encrypted_path() {
    assert!(is_encrypted_path(&PathBuf::from("secret.txt.cott.age")));
    assert!(!is_encrypted_path(&PathBuf::from("secret.txt")));
}

#[test]
fn test_to_decrypted_path() {
    let encrypted_path = PathBuf::from("secret.txt.cott.age");
    let decrypted_path = to_decrypted_path(&encrypted_path);
    assert_eq!(decrypted_path, Some(PathBuf::from("secret.txt")));

    let non_encrypted_path = PathBuf::from("secret.txt");
    assert_eq!(to_decrypted_path(&non_encrypted_path), None);

    let dotenv_encrypted_path = PathBuf::from(".env.cott.age");
    let decrypted_dotenv_path = to_decrypted_path(&dotenv_encrypted_path);
    assert_eq!(decrypted_dotenv_path, Some(PathBuf::from(".env")));

    let double_extension_path = PathBuf::from("archive.tar.gz.cott.age");
    let decrypted_double_extension_path = to_decrypted_path(&double_extension_path);
    assert_eq!(
        decrypted_double_extension_path,
        Some(PathBuf::from("archive.tar.gz"))
    );

    let double_encrypted_path = PathBuf::from("secret.txt.cott.age.cott.age");
    let decrypted_double_encrypted_path = to_decrypted_path(&double_encrypted_path);
    assert_eq!(
        decrypted_double_encrypted_path,
        Some(PathBuf::from("secret.txt.cott.age"))
    );
}

#[test]
fn test_add_to_gitignore() {
    let parent_dir = assert_fs::TempDir::new().unwrap();
    let git_dir = parent_dir.path().join(".git");
    std::fs::create_dir_all(&git_dir).unwrap();

    let subdir = parent_dir.path().join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();

    let parent_gitignore = parent_dir.path().join(".gitignore");
    let subdir_gitignore = subdir.join(".gitignore");

    assert!(!parent_gitignore.exists());
    assert!(!subdir_gitignore.exists());

    let parent_secret = parent_dir.path().join("secret.txt");
    std::fs::write(&parent_secret, "secret").unwrap();

    let subdir_secret = subdir.join("subsecret.txt");
    std::fs::write(&subdir_secret, "subsecret").unwrap();

    // Test adding to parent .gitignore
    let added_path = append_to_gitignore_if_absent(&parent_secret, false)
        .unwrap()
        .unwrap();
    assert_eq!(added_path, parent_gitignore);

    let parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
    assert!(parent_content.contains("/secret.txt"));

    // Test adding to parent .gitignore when subdir .gitignore doesn't exist
    let added_subpath = append_to_gitignore_if_absent(&subdir_secret, false)
        .unwrap()
        .unwrap();
    assert_eq!(added_subpath, parent_gitignore);

    let parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
    assert!(parent_content.contains("/subdir/subsecret.txt"));

    // Test adding to subdir .gitignore
    std::fs::write(&subdir_gitignore, "").unwrap();
    let added_subpath_to_subdir = append_to_gitignore_if_absent(&subdir_secret, false).unwrap();

    assert_eq!(added_subpath_to_subdir, Some(subdir_gitignore.clone()));

    let subdir_content = std::fs::read_to_string(&subdir_gitignore).unwrap();
    assert!(subdir_content.contains("/subsecret.txt"));

    // Check duplicates are not added
    let duplicate_parent_add = append_to_gitignore_if_absent(&parent_secret, false).unwrap();
    let duplicate_subdir_add = append_to_gitignore_if_absent(&subdir_secret, false).unwrap();
    let updated_parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
    let updated_subdir_content = std::fs::read_to_string(&subdir_gitignore).unwrap();

    assert_eq!(duplicate_parent_add, None);
    assert_eq!(duplicate_subdir_add, None);

    assert_eq!(parent_content, updated_parent_content);
    assert_eq!(subdir_content, updated_subdir_content);
}

#[test]
fn test_add_line_if_absent() {
    let temp_file = assert_fs::NamedTempFile::new("test.txt").unwrap();
    let path = temp_file.path();

    // Test adding a line to an empty file
    append_line_if_absent(path, "line1", false).unwrap();
    let content = std::fs::read_to_string(path).unwrap();
    assert_eq!(content, "line1\n");

    // Test adding the same line again (should not be added)
    append_line_if_absent(path, "line1", false).unwrap();
    let content = std::fs::read_to_string(path).unwrap();
    assert_eq!(content, "line1\n");

    // Test adding a different line
    append_line_if_absent(path, "line2", false).unwrap();
    let content = std::fs::read_to_string(path).unwrap();
    assert_eq!(content, "line1\nline2\n");

    // Test newline is added if file doesn't end with newline
    std::fs::write(path, "line1").unwrap();
    append_line_if_absent(path, "line2", false).unwrap();
    let content = std::fs::read_to_string(path).unwrap();
    assert_eq!(content, "line1\nline2\n");
}

#[test]
fn test_project_clean() -> anyhow::Result<()> {
    let root = assert_fs::TempDir::new()?;
    std::env::set_current_dir(root.path())?;

    let cottage_dir = root.path().join(".cottage");
    std::fs::create_dir_all(&cottage_dir)?;
    std::fs::create_dir_all(root.path().join(".git"))?;

    let proj = Project::load()?;
    assert!(cottage_dir.exists());

    // Dry run
    proj.clean(true)?;
    assert!(cottage_dir.exists());

    // Actual clean
    proj.clean(false)?;
    assert!(!cottage_dir.exists());

    Ok(())
}
