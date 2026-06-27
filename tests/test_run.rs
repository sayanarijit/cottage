use assert_fs::prelude::*;
use std::process::Command;

#[test]
fn test_run_command() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create secret
    temp.child("secret.txt").write_str("my secret").unwrap();

    // Encrypt
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete decrypted
    std::fs::remove_file(temp.path().join("secret.txt")).unwrap();

    // Run
    let output = Command::new(bin_path)
        .arg("run")
        .arg("cat")
        .arg("secret.txt")
        .current_dir(temp.path())
        .output()
        .unwrap();

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
    // The output might contain "decrypting..." etc. if not silent.
    // But stdout should contain "my secret" from cat.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my secret"));
    assert!(!temp.path().join("secret.txt").exists());
}

#[test]
fn test_keygen_command() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init project
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    let identity_path = temp.path().join(".cottage/identity");
    let user1_recipient_path = temp.path().join(".cottage/recipients/user1");
    let user2_recipient_path = temp.path().join(".cottage/recipients/user2");

    // Running keygen without --force should fail if identity already exists
    let output = Command::new(bin_path)
        .arg("keygen")
        .arg("-n")
        .arg("user1")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    // Running keygen with --force should succeed and overwrite
    let output = Command::new(bin_path)
        .arg("keygen")
        .arg("-n")
        .arg("user1")
        .arg("--force")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(identity_path.exists());
    assert!(user1_recipient_path.exists());

    // Delete keys
    std::fs::remove_file(&identity_path).unwrap();
    std::fs::remove_dir_all(temp.path().join(".cottage/recipients")).unwrap();

    // Running keygen when keys are missing should succeed without --force
    let output = Command::new(bin_path)
        .arg("keygen")
        .arg("-n")
        .arg("user2")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(identity_path.exists());
    assert!(user2_recipient_path.exists());
}

#[test]
fn test_run_command_nested() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init at root
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create nested directory
    let nested_dir = temp.path().join("nested/dir");
    std::fs::create_dir_all(&nested_dir).unwrap();

    // Create secret inside nested directory
    std::fs::write(nested_dir.join("secret.txt"), "my secret").unwrap();

    // Encrypt from nested directory
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Delete decrypted secret
    std::fs::remove_file(nested_dir.join("secret.txt")).unwrap();

    // Run from nested directory
    let output = Command::new(bin_path)
        .arg("run")
        .arg("cat")
        .arg("secret.txt")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my secret"));
    assert!(!nested_dir.join("secret.txt").exists());
}

#[test]
fn test_keygen_command_nested() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init project
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create nested directory
    let nested_dir = temp.path().join("nested/dir");
    std::fs::create_dir_all(&nested_dir).unwrap();

    let identity_path = temp.path().join(".cottage/identity");
    let user1_recipient_path = temp.path().join(".cottage/recipients/user1");
    let user2_recipient_path = temp.path().join(".cottage/recipients/user2");

    // Running keygen without --force should fail if identity already exists
    let output = Command::new(bin_path)
        .arg("keygen")
        .arg("-n")
        .arg("user1")
        .current_dir(&nested_dir)
        .output()
        .unwrap();
    assert!(!output.status.success());

    // Running keygen with --force should succeed and overwrite
    let output = Command::new(bin_path)
        .arg("keygen")
        .arg("-n")
        .arg("user1")
        .arg("--force")
        .current_dir(&nested_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(identity_path.exists());
    assert!(user1_recipient_path.exists());

    // Delete keys
    std::fs::remove_file(&identity_path).unwrap();
    std::fs::remove_dir_all(temp.path().join(".cottage/recipients")).unwrap();

    // Running keygen when keys are missing should succeed without --force
    let output = Command::new(bin_path)
        .arg("keygen")
        .arg("-n")
        .arg("user2")
        .current_dir(&nested_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(identity_path.exists());
    assert!(user2_recipient_path.exists());
}

