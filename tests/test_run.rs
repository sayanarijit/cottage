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

#[test]
fn test_encrypt_clean_already_present() {
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

    // Encrypt with --clean
    let output = Command::new(bin_path)
        .arg("encrypt")
        .arg("secret.txt")
        .arg("--clean")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());
}

#[test]
fn test_edit_clean_already_present() {
    use std::io::Write;
    use std::process::Stdio;

    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create secret file
    temp.child("secret.txt").write_str("initial secret").unwrap();

    // Edit with --clean passing decrypted path and piping new content
    let mut child = Command::new(bin_path)
        .arg("edit")
        .arg("secret.txt")
        .arg("--clean")
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"updated secret")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(!temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());

    // Recreate secret.txt, then edit with --clean passing encrypted path
    temp.child("secret.txt").write_str("another secret").unwrap();

    let mut child = Command::new(bin_path)
        .arg("edit")
        .arg("secret.txt.cott.age")
        .arg("--clean")
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"final secret")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(!temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());
}

