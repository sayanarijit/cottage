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

#[test]
fn test_pull_push_requires() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create required secret
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Remove required_secret.txt (we want cottage to decrypt it automatically)
    std::fs::remove_file(temp.path().join("required_secret.txt")).unwrap();

    // Create main secret
    temp.child("secret1.txt").write_str("dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull
    std::fs::remove_file(temp.path().join("secret1.txt")).unwrap();

    // Configure cottage.toml with requires
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["required_secret.txt"]

[upstream.my-upstream.pull]
shell = "bash"
script = """
if [ ! -f "required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
echo '{"SECRET": "pulled value"}'
"""

[upstream.my-upstream.push]
shell = "bash"
script = """
if [ ! -f "required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
"""
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
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

    // Ensure required_secret.txt does not exist before running pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PULL
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

    // Ensure required_secret.txt was cleaned up after pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PUSH
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

    // Ensure required_secret.txt was cleaned up after push
    assert!(!temp.path().join("required_secret.txt").exists());
}

#[test]
fn test_pull_push_auto_require_vars() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create required secret
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Remove required_secret.txt (we want cottage to decrypt it automatically)
    std::fs::remove_file(temp.path().join("required_secret.txt")).unwrap();

    // Create main secret
    temp.child("secret1.txt").write_str("dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull
    std::fs::remove_file(temp.path().join("secret1.txt")).unwrap();

    // Configure cottage.toml with vars instead of requires
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
vars = { MY_SECRET_VAR = "required_secret.txt" }

[upstream.my-upstream.pull]
shell = "bash"
script = """
if [ ! -f "required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
echo '{"SECRET": "pulled value"}'
"""

[upstream.my-upstream.push]
shell = "bash"
script = """
if [ ! -f "required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
"""
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
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

    // Ensure required_secret.txt does not exist before running pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PULL
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

    // Ensure required_secret.txt was cleaned up after pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PUSH
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

    // Ensure required_secret.txt was cleaned up after push
    assert!(!temp.path().join("required_secret.txt").exists());
}

#[test]
fn test_pull_push_auto_require_vars_operation_specific() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create required secret
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Remove required_secret.txt (we want cottage to decrypt it automatically)
    std::fs::remove_file(temp.path().join("required_secret.txt")).unwrap();

    // Create main secret
    temp.child("secret1.txt").write_str("dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull
    std::fs::remove_file(temp.path().join("secret1.txt")).unwrap();

    // Configure cottage.toml with vars in pull and push sections
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]

[upstream.my-upstream.pull]
vars = { MY_SECRET_VAR = "required_secret.txt" }
shell = "bash"
script = """
if [ ! -f "required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
echo '{"SECRET": "pulled value"}'
"""

[upstream.my-upstream.push]
vars = { MY_SECRET_VAR = "required_secret.txt" }
shell = "bash"
script = """
if [ ! -f "required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
"""
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
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

    // Ensure required_secret.txt does not exist before running pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PULL
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

    // Ensure required_secret.txt was cleaned up after pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PUSH
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

    // Ensure required_secret.txt was cleaned up after push
    assert!(!temp.path().join("required_secret.txt").exists());
}

#[test]
fn test_pull_push_dirty_requirements() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create required secret and encrypt it
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Make required_secret.txt dirty by writing new content
    temp.child("required_secret.txt")
        .write_str("important key - modified but not encrypted")
        .unwrap();

    // Create main secret
    temp.child("secret1.txt").write_str("dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull for the main secret itself
    std::fs::remove_file(temp.path().join("secret1.txt")).unwrap();

    // Configure cottage.toml with requires
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["required_secret.txt"]

[upstream.my-upstream.pull]
shell = "bash"
script = "echo '{\"SECRET\": \"pulled value\"}'"

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
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

    // PULL should fail because required_secret.txt is dirty
    let output_pull = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let stderr_pull = String::from_utf8_lossy(&output_pull.stderr);
    assert!(
        !output_pull.status.success(),
        "PULL succeeded but should have failed since requirement is dirty"
    );
    assert!(
        stderr_pull
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_pull
    );

    // PUSH should fail because required_secret.txt is dirty
    let output_push = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let stderr_push = String::from_utf8_lossy(&output_push.stderr);
    assert!(
        !output_push.status.success(),
        "PUSH succeeded but should have failed since requirement is dirty"
    );
    assert!(
        stderr_push
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_push
    );
}

#[test]
fn test_pull_push_dirty_requirements_via_vars() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create required secret and encrypt it
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Make required_secret.txt dirty by writing new content
    temp.child("required_secret.txt")
        .write_str("important key - modified but not encrypted")
        .unwrap();

    // Create main secret
    temp.child("secret1.txt").write_str("dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull for the main secret itself
    std::fs::remove_file(temp.path().join("secret1.txt")).unwrap();

    // Configure cottage.toml with vars referencing required_secret.txt
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
vars = { MY_VAR = "required_secret.txt" }

[upstream.my-upstream.pull]
shell = "bash"
script = "echo '{\"SECRET\": \"pulled value\"}'"

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
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

    // PULL should fail because required_secret.txt is dirty
    let output_pull = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let stderr_pull = String::from_utf8_lossy(&output_pull.stderr);
    assert!(
        !output_pull.status.success(),
        "PULL succeeded but should have failed since requirement from vars is dirty"
    );
    assert!(
        stderr_pull
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_pull
    );

    // PUSH should fail because required_secret.txt is dirty
    let output_push = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let stderr_push = String::from_utf8_lossy(&output_push.stderr);
    assert!(
        !output_push.status.success(),
        "PUSH succeeded but should have failed since requirement from vars is dirty"
    );
    assert!(
        stderr_push
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_push
    );
}

#[test]
fn test_pull_push_dirty_requirements_encrypted_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create required secret and encrypt it
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Make required_secret.txt dirty by writing new content
    temp.child("required_secret.txt")
        .write_str("important key - modified but not encrypted")
        .unwrap();

    // Create main secret
    temp.child("secret1.txt").write_str("dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull for the main secret itself
    std::fs::remove_file(temp.path().join("secret1.txt")).unwrap();

    // Configure cottage.toml with requires pointing to the encrypted file
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["required_secret.txt.cott.age"]

[upstream.my-upstream.pull]
shell = "bash"
script = "echo '{\"SECRET\": \"pulled value\"}'"

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
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

    // PULL should fail because required_secret.txt is dirty
    let output_pull = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let stderr_pull = String::from_utf8_lossy(&output_pull.stderr);
    assert!(
        !output_pull.status.success(),
        "PULL succeeded but should have failed since requirement is dirty"
    );
    assert!(
        stderr_pull
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_pull
    );
}

#[test]
fn test_pull_push_from_subdirectory() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init project at project root
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create subdirectory
    let subdir = temp.path().join("subdir");
    std::fs::create_dir_all(&subdir.join("secrets")).unwrap();

    // Create required secret in subdir
    let req_secret_path = subdir.join("secrets").join("req_secret.txt");
    std::fs::write(&req_secret_path, "important key").unwrap();

    Command::new(bin_path)
        .arg("encrypt")
        .arg("subdir/secrets/req_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Remove required decrypted file
    std::fs::remove_file(&req_secret_path).unwrap();

    // Create main secret in subdir
    let secret_path = subdir.join("secrets").join("secret.txt");
    std::fs::write(&secret_path, "secret content").unwrap();

    Command::new(bin_path)
        .arg("encrypt")
        .arg("subdir/secrets/secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Configure cottage.toml at project root with relative requires path relative to project root
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["./subdir/secrets/req_secret.txt"]

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret.txt.cott.toml
    let metadata_path = subdir.join("secrets").join("secret.txt.cott.toml");
    let mut metadata: toml::Value =
        toml::from_str(&std::fs::read_to_string(&metadata_path).unwrap()).unwrap();
    metadata.as_table_mut().unwrap().insert(
        "upstream".to_string(),
        toml::from_str(
            r#"
        [my-upstream]
        push = true
    "#,
        )
        .unwrap(),
    );
    std::fs::write(&metadata_path, toml::to_string(&metadata).unwrap()).unwrap();

    // PUSH from subdir directory, referencing the secret.txt.cott.age relatively
    let output = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .arg("./secrets/secret.txt.cott.age")
        .current_dir(&subdir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "PUSH failed from subdirectory. Stderr: {}\nStdout: {}",
        stderr,
        stdout
    );
}

#[test]
fn test_pull_push_with_config_nested() {
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

    // Create cottage.toml with upstream configuration at project root
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

    // Create secret 1 in nested directory
    std::fs::write(nested_dir.join("secret1.txt"), "secret 1 content").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Add upstream to secret1.txt.cott.toml
    let metadata_path1 = nested_dir.join("secret1.txt.cott.toml");
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

    // Create secret 2 in nested directory
    std::fs::write(nested_dir.join("secret2.txt"), "secret 2 content").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret2.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Add upstream to secret2.txt.cott.toml
    let metadata_path2 = nested_dir.join("secret2.txt.cott.toml");
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
    std::fs::remove_file(nested_dir.join("secret1.txt")).unwrap();
    std::fs::remove_file(nested_dir.join("secret2.txt")).unwrap();

    // PULL from nested directory
    let output = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PULL failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));
    assert!(!stdout.contains("secret2.txt.cott.age"));

    // PUSH from nested directory
    let output = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PUSH failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));
    assert!(!stdout.contains("secret2.txt.cott.age"));
}

#[test]
fn test_pull_push_requires_nested() {
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

    // Create required secret at root
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Remove required_secret.txt (we want cottage to decrypt it automatically)
    std::fs::remove_file(temp.path().join("required_secret.txt")).unwrap();

    // Create main secret in nested directory
    std::fs::write(nested_dir.join("secret1.txt"), "dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull
    std::fs::remove_file(nested_dir.join("secret1.txt")).unwrap();

    // Configure cottage.toml with requires
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["required_secret.txt"]

[upstream.my-upstream.pull]
shell = "bash"
script = """
if [ ! -f "../../required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat ../../required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
echo '{"SECRET": "pulled value"}'
"""

[upstream.my-upstream.push]
shell = "bash"
script = """
if [ ! -f "../../required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat ../../required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
"""
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
    let metadata_path1 = nested_dir.join("secret1.txt.cott.toml");
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

    // Ensure required_secret.txt does not exist before running pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PULL from nested directory
    let output = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PULL failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));

    // Ensure required_secret.txt was cleaned up after pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PUSH from nested directory
    let output = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PUSH failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));

    // Ensure required_secret.txt was cleaned up after push
    assert!(!temp.path().join("required_secret.txt").exists());
}

#[test]
fn test_pull_push_auto_require_vars_nested() {
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

    // Create required secret at root
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Remove required_secret.txt (we want cottage to decrypt it automatically)
    std::fs::remove_file(temp.path().join("required_secret.txt")).unwrap();

    // Create main secret inside nested directory
    std::fs::write(nested_dir.join("secret1.txt"), "dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull
    std::fs::remove_file(nested_dir.join("secret1.txt")).unwrap();

    // Configure cottage.toml with vars instead of requires
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
vars = { MY_SECRET_VAR = "required_secret.txt" }

[upstream.my-upstream.pull]
shell = "bash"
script = """
if [ ! -f "../../required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat ../../required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
echo '{"SECRET": "pulled value"}'
"""

[upstream.my-upstream.push]
shell = "bash"
script = """
if [ ! -f "../../required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat ../../required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
"""
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
    let metadata_path1 = nested_dir.join("secret1.txt.cott.toml");
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

    // Ensure required_secret.txt does not exist before running pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PULL from nested directory
    let output = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PULL failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));

    // Ensure required_secret.txt was cleaned up after pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PUSH from nested directory
    let output = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PUSH failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));

    // Ensure required_secret.txt was cleaned up after push
    assert!(!temp.path().join("required_secret.txt").exists());
}

#[test]
fn test_pull_push_auto_require_vars_operation_specific_nested() {
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

    // Create required secret at root
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Remove required_secret.txt (we want cottage to decrypt it automatically)
    std::fs::remove_file(temp.path().join("required_secret.txt")).unwrap();

    // Create main secret inside nested directory
    std::fs::write(nested_dir.join("secret1.txt"), "dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull
    std::fs::remove_file(nested_dir.join("secret1.txt")).unwrap();

    // Configure cottage.toml with vars in pull and push sections
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]

[upstream.my-upstream.pull]
vars = { MY_SECRET_VAR = "required_secret.txt" }
shell = "bash"
script = """
if [ ! -f "../../required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat ../../required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
echo '{"SECRET": "pulled value"}'
"""

[upstream.my-upstream.push]
vars = { MY_SECRET_VAR = "required_secret.txt" }
shell = "bash"
script = """
if [ ! -f "../../required_secret.txt" ]; then
    echo "required_secret.txt not found!" >&2
    exit 1
fi
content=$(cat ../../required_secret.txt)
if [ "$content" != "important key" ]; then
    echo "Incorrect content in required_secret.txt: $content" >&2
    exit 1
fi
if [ "$MY_SECRET_VAR" != "required_secret.txt" ]; then
    echo "MY_SECRET_VAR not set to required_secret.txt: $MY_SECRET_VAR" >&2
    exit 1
fi
"""
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
    let metadata_path1 = nested_dir.join("secret1.txt.cott.toml");
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

    // Ensure required_secret.txt does not exist before running pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PULL from nested directory
    let output = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PULL failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));

    // Ensure required_secret.txt was cleaned up after pull
    assert!(!temp.path().join("required_secret.txt").exists());

    // PUSH from nested directory
    let output = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "PUSH failed: {}", stderr);
    assert!(stdout.contains("secret1.txt.cott.age"));

    // Ensure required_secret.txt was cleaned up after push
    assert!(!temp.path().join("required_secret.txt").exists());
}

#[test]
fn test_pull_push_dirty_requirements_nested() {
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

    // Create required secret and encrypt it at root
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Make required_secret.txt dirty by writing new content at root
    temp.child("required_secret.txt")
        .write_str("important key - modified but not encrypted")
        .unwrap();

    // Create main secret inside nested directory
    std::fs::write(nested_dir.join("secret1.txt"), "dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull for the main secret itself
    std::fs::remove_file(nested_dir.join("secret1.txt")).unwrap();

    // Configure cottage.toml with requires
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["required_secret.txt"]

[upstream.my-upstream.pull]
shell = "bash"
script = "echo '{\"SECRET\": \"pulled value\"}'"

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
    let metadata_path1 = nested_dir.join("secret1.txt.cott.toml");
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

    // PULL should fail because required_secret.txt is dirty
    let output_pull = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stderr_pull = String::from_utf8_lossy(&output_pull.stderr);
    assert!(
        !output_pull.status.success(),
        "PULL succeeded but should have failed since requirement is dirty"
    );
    assert!(
        stderr_pull
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_pull
    );

    // PUSH should fail because required_secret.txt is dirty
    let output_push = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stderr_push = String::from_utf8_lossy(&output_push.stderr);
    assert!(
        !output_push.status.success(),
        "PUSH succeeded but should have failed since requirement is dirty"
    );
    assert!(
        stderr_push
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_push
    );
}

#[test]
fn test_pull_push_dirty_requirements_via_vars_nested() {
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

    // Create required secret and encrypt it at root
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Make required_secret.txt dirty by writing new content at root
    temp.child("required_secret.txt")
        .write_str("important key - modified but not encrypted")
        .unwrap();

    // Create main secret inside nested directory
    std::fs::write(nested_dir.join("secret1.txt"), "dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull for the main secret itself
    std::fs::remove_file(nested_dir.join("secret1.txt")).unwrap();

    // Configure cottage.toml with vars referencing required_secret.txt
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
vars = { MY_VAR = "required_secret.txt" }

[upstream.my-upstream.pull]
shell = "bash"
script = "echo '{\"SECRET\": \"pulled value\"}'"

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
    let metadata_path1 = nested_dir.join("secret1.txt.cott.toml");
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

    // PULL should fail because required_secret.txt is dirty
    let output_pull = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stderr_pull = String::from_utf8_lossy(&output_pull.stderr);
    assert!(
        !output_pull.status.success(),
        "PULL succeeded but should have failed since requirement from vars is dirty"
    );
    assert!(
        stderr_pull
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_pull
    );

    // PUSH should fail because required_secret.txt is dirty
    let output_push = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stderr_push = String::from_utf8_lossy(&output_push.stderr);
    assert!(
        !output_push.status.success(),
        "PUSH succeeded but should have failed since requirement from vars is dirty"
    );
    assert!(
        stderr_push
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_push
    );
}

#[test]
fn test_pull_push_dirty_requirements_encrypted_path_nested() {
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

    // Create required secret and encrypt it at root
    temp.child("required_secret.txt")
        .write_str("important key")
        .unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("required_secret.txt")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Make required_secret.txt dirty by writing new content at root
    temp.child("required_secret.txt")
        .write_str("important key - modified but not encrypted")
        .unwrap();

    // Create main secret inside nested directory
    std::fs::write(nested_dir.join("secret1.txt"), "dummy value").unwrap();
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secret1.txt")
        .current_dir(&nested_dir)
        .status()
        .unwrap();

    // Delete secret1.txt to avoid "dirty" error on pull for the main secret itself
    std::fs::remove_file(nested_dir.join("secret1.txt")).unwrap();

    // Configure cottage.toml with requires pointing to the encrypted file at root
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["required_secret.txt.cott.age"]

[upstream.my-upstream.pull]
shell = "bash"
script = "echo '{\"SECRET\": \"pulled value\"}'"

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret1.txt.cott.toml
    let metadata_path1 = nested_dir.join("secret1.txt.cott.toml");
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

    // PULL should fail because required_secret.txt is dirty
    let output_pull = Command::new(bin_path)
        .arg("pull")
        .arg("my-upstream")
        .current_dir(&nested_dir)
        .output()
        .unwrap();

    let stderr_pull = String::from_utf8_lossy(&output_pull.stderr);
    assert!(
        !output_pull.status.success(),
        "PULL succeeded but should have failed since requirement is dirty"
    );
    assert!(
        stderr_pull
            .contains("required_secret.txt is dirty, please run `ctg sync` or `ctg encrypt` first"),
        "Unexpected error: {}",
        stderr_pull
    );
}

#[test]
fn test_pull_push_from_subdirectory_nested() {
    let temp = assert_fs::TempDir::new().unwrap();
    let bin_path = env!("CARGO_BIN_EXE_ctg");

    // Init project at project root
    Command::new(bin_path)
        .arg("init")
        .current_dir(temp.path())
        .status()
        .unwrap();

    // Create subdirectory
    let subdir = temp.path().join("subdir");
    let secrets_dir = subdir.join("secrets");
    std::fs::create_dir_all(&secrets_dir).unwrap();

    // Create required secret in secrets_dir
    let req_secret_path = secrets_dir.join("req_secret.txt");
    std::fs::write(&req_secret_path, "important key").unwrap();

    // Encrypt from subdir using relative path to subdir
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secrets/req_secret.txt")
        .current_dir(&subdir)
        .status()
        .unwrap();

    // Remove required decrypted file
    std::fs::remove_file(&req_secret_path).unwrap();

    // Create main secret in secrets_dir
    let secret_path = secrets_dir.join("secret.txt");
    std::fs::write(&secret_path, "secret content").unwrap();

    // Encrypt from subdir using relative path to subdir
    Command::new(bin_path)
        .arg("encrypt")
        .arg("secrets/secret.txt")
        .current_dir(&subdir)
        .status()
        .unwrap();

    // Configure cottage.toml at project root with relative requires path relative to project root
    temp.child("cottage.toml")
        .write_str(
            r#"
[upstream.my-upstream]
requires = ["./subdir/secrets/req_secret.txt"]

[upstream.my-upstream.push]
shell = "bash"
script = "cat > /dev/null"
"#,
        )
        .unwrap();

    // Add upstream config to secret.txt.cott.toml
    let metadata_path = secrets_dir.join("secret.txt.cott.toml");
    let mut metadata: toml::Value =
        toml::from_str(&std::fs::read_to_string(&metadata_path).unwrap()).unwrap();
    metadata.as_table_mut().unwrap().insert(
        "upstream".to_string(),
        toml::from_str(
            r#"
        [my-upstream]
        push = true
    "#,
        )
        .unwrap(),
    );
    std::fs::write(&metadata_path, toml::to_string(&metadata).unwrap()).unwrap();

    // PUSH from secrets_dir directory (even deeper nested), referencing the secret.txt.cott.age relatively (which is in the same directory, so just "secret.txt.cott.age")
    let output = Command::new(bin_path)
        .arg("push")
        .arg("my-upstream")
        .arg("secret.txt.cott.age")
        .current_dir(&secrets_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "PUSH failed from deeply nested subdirectory. Stderr: {}\nStdout: {}",
        stderr,
        stdout
    );
}

