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
    temp.child("secret.txt")
        .write_str("initial secret")
        .unwrap();

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
    temp.child("secret.txt")
        .write_str("another secret")
        .unwrap();

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

#[test]
fn test_edit_auto_clean_when_not_present() {
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

    // Create and encrypt secret
    temp.child("secret.txt")
        .write_str("initial secret")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete decrypted file (not present before edit)
    std::fs::remove_file(temp.path().join("secret.txt")).unwrap();
    assert!(!temp.path().join("secret.txt").exists());

    // Edit WITHOUT --clean by targeting secret.txt.cott.age
    let mut child = Command::new(bin_path)
        .arg("edit")
        .arg("secret.txt.cott.age")
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"updated secret without clean")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    // Since secret.txt was not present before edit, it must be AUTO-CLEANED even without --clean
    assert!(
        !temp.path().join("secret.txt").exists(),
        "secret.txt was not present before edit and should have been auto-cleaned"
    );
    assert!(temp.path().join("secret.txt.cott.age").exists());

    // Verify the encrypted secret was indeed updated
    let output = Command::new(bin_path)
        .arg("run")
        .arg("cat")
        .arg("secret.txt")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("updated secret without clean"));
}

#[test]
fn test_edit_retains_when_already_present_without_clean() {
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

    // Create and encrypt secret
    temp.child("secret.txt")
        .write_str("initial secret")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // secret.txt is already present
    assert!(temp.path().join("secret.txt").exists());

    // Edit WITHOUT --clean
    let mut child = Command::new(bin_path)
        .arg("edit")
        .arg("secret.txt")
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"updated secret already present")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    // Since secret.txt was already present and --clean was NOT passed, it MUST RETAIN
    assert!(
        temp.path().join("secret.txt").exists(),
        "secret.txt was already present and should have been retained"
    );
    let content = std::fs::read_to_string(temp.path().join("secret.txt")).unwrap();
    assert_eq!(content, "updated secret already present");
}

#[test]
fn test_run_clean_already_present() {
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

    // Encrypt (secret.txt and secret.txt.cott.age both exist)
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    assert!(temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());

    // Run WITHOUT --clean: secret.txt should remain on disk
    let output = Command::new(bin_path)
        .arg("run")
        .arg("cat")
        .arg("secret.txt")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("my secret"));
    assert!(temp.path().join("secret.txt").exists());

    // Run WITH --clean: secret.txt should be deleted
    let output = Command::new(bin_path)
        .arg("run")
        .arg("--clean")
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("my secret"));
    assert!(!temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());

    // Recreate secret.txt and test with encrypted path passed to args
    temp.child("secret.txt")
        .write_str("another secret")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    let output = Command::new(bin_path)
        .arg("run")
        .arg("--clean")
        .arg("cat")
        .arg("secret.txt.cott.age")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("another secret"));
    assert!(!temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());
}

#[test]
fn test_ctgx_clean_already_present() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path_ctg = env!("CARGO_BIN_EXE_ctg");
    let bin_path_ctgx = env!("CARGO_BIN_EXE_ctgx");

    // Init
    Command::new(bin_path_ctg)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create secret
    temp.child("secret.txt").write_str("ctgx secret").unwrap();

    // Encrypt
    Command::new(bin_path_ctg)
        .arg("encrypt")
        .arg("secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    assert!(temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());

    // Run ctgx with --clean
    let output = Command::new(bin_path_ctgx)
        .arg("--clean")
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("ctgx secret"));
    assert!(!temp.path().join("secret.txt").exists());
    assert!(temp.path().join("secret.txt.cott.age").exists());
}

#[test]
fn test_run_auto_clean_only_when_not_present_or_clean_flag() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create secret1 and secret2
    temp.child("secret1.txt").write_str("secret one").unwrap();
    temp.child("secret2.txt").write_str("secret two").unwrap();

    // Encrypt both
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .arg("secret2.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // secret1.txt is kept (already present)
    // secret2.txt is removed (not present before run)
    std::fs::remove_file(temp.path().join("secret2.txt")).unwrap();

    assert!(temp.path().join("secret1.txt").exists());
    assert!(!temp.path().join("secret2.txt").exists());

    // Run WITHOUT --clean
    let output = Command::new(bin_path)
        .arg("run")
        .arg("cat")
        .arg("secret2.txt")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("secret two"));

    // secret1.txt was already present and --clean was NOT passed => MUST STILL EXIST
    assert!(
        temp.path().join("secret1.txt").exists(),
        "secret1.txt was already present and should not have been cleaned"
    );

    // secret2.txt was NOT present before run and was temporarily decrypted => MUST BE AUTO-CLEANED
    assert!(
        !temp.path().join("secret2.txt").exists(),
        "secret2.txt was temporarily decrypted and should have been auto-cleaned"
    );

    // Recreate secret2.txt and encrypt it so it is not dirty
    temp.child("secret2.txt").write_str("secret two").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret2.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();
    assert!(temp.path().join("secret1.txt").exists());
    assert!(temp.path().join("secret2.txt").exists());

    // Run WITH --clean targeting secret1.txt
    let output = Command::new(bin_path)
        .arg("run")
        .arg("--clean")
        .arg("cat")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("secret one"));

    // secret1.txt should be deleted because --clean was passed
    assert!(
        !temp.path().join("secret1.txt").exists(),
        "secret1.txt should be cleaned because --clean was passed"
    );
    // secret2.txt was not in input args, so it should still exist
    assert!(
        temp.path().join("secret2.txt").exists(),
        "secret2.txt was not targeted and should remain"
    );

    // Run WITH --clean on the entire directory (no file arg)
    let output = Command::new(bin_path)
        .arg("run")
        .arg("--clean")
        .arg("true")
        .current_dir(temp.path())
        .output()
        .unwrap();

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
    // Now secret2.txt should also be cleaned
    assert!(
        !temp.path().join("secret2.txt").exists(),
        "secret2.txt should be cleaned after project-wide clean"
    );
}
