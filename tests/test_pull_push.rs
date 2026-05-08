use assert_fs::prelude::*;
use std::process::Command;

#[test]
fn test_pull_push_with_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create cottage.toml with upstream configuration
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream.pull]
script = "echo '{\"SECRET\": \"pulled content\"}'"

[upstream.my-upstream.push]
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Create secret 1 with 'my-upstream'
    temp.child("secret1.txt")
        .write_str("secret 1 content")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Add upstream to secret1.txt.cott.toml
    let metadata_path1 = temp.path().join("secret1.txt.cott.toml");
    let mut metadata1: toml::Value =
        toml::from_str(&std::fs::read_to_string(&metadata_path1).unwrap()).unwrap();
    metadata1.as_table_mut().unwrap().insert(
        "upstream".to_string(),
        toml::from_str(
            r#"
        [my-upstream]
        pull = true
        push = true
    "#,
        )
        .unwrap(),
    );
    std::fs::write(&metadata_path1, toml::to_string(&metadata1).unwrap()).unwrap();

    // Create secret 2 with 'other-upstream' (not configured in cottage.toml)
    temp.child("secret2.txt")
        .write_str("secret 2 content")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret2.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Add upstream to secret2.txt.cott.toml
    let metadata_path2 = temp.path().join("secret2.txt.cott.toml");
    let mut metadata2: toml::Value =
        toml::from_str(&std::fs::read_to_string(&metadata_path2).unwrap()).unwrap();
    metadata2.as_table_mut().unwrap().insert(
        "upstream".to_string(),
        toml::from_str(
            r#"
        [other-upstream]
        pull = true
        push = true
    "#,
        )
        .unwrap(),
    );
    std::fs::write(&metadata_path2, toml::to_string(&metadata2).unwrap()).unwrap();

    // Delete decrypted to avoid "dirty" errors when pulling/pushing
    std::fs::remove_file(temp.path().join("secret1.txt")).unwrap();
    std::fs::remove_file(temp.path().join("secret2.txt")).unwrap();

    // PULL
    // Try to pull 'my-upstream' at project root
    let output = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PULL failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));
    assert!(!stdout.contains("secret2.txt.cott.age"));

    // PUSH
    // Try to push 'my-upstream' at project root
    let output = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PUSH failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));
    assert!(!stdout.contains("secret2.txt.cott.age"));
}
