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
