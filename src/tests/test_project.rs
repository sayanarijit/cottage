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

#[test]
fn test_upstream_config_resolved_direct() {
    use crate::project::{PullPushConfig, UpstreamConfig};
    use indexmap::{IndexMap, IndexSet};
    use std::path::PathBuf;

    let mut vars = IndexMap::new();
    vars.insert("k1".to_string(), "v1_top".to_string());
    vars.insert("k2".to_string(), "v2_top".to_string());

    let mut requires = IndexSet::new();
    requires.insert(PathBuf::from("req_top"));

    let config = UpstreamConfig {
        cwd: Some(true),
        envfile: Some(PathBuf::from("top.env")),
        vars: Some(vars),
        requires: Some(requires),
        shell: Some("top_shell".to_string()),
        plugin: Some("top_plugin".to_string()),
        pull: Some(PullPushConfig {
            cwd: None,
            envfile: None,
            vars: Some({
                let mut map = IndexMap::new();
                map.insert("k2".to_string(), "v2_pull".to_string());
                map
            }),
            requires: Some({
                let mut set = IndexSet::new();
                set.insert(PathBuf::from("req_pull"));
                set
            }),
            shell: None,
            script: Some("pull_script".to_string()),
            plugin: None,
        }),
        push: None,
    };

    let resolved = config.resolved();

    // Verify pull
    let pull = resolved.pull.as_ref().unwrap();
    assert_eq!(pull.cwd, Some(true)); // Inherited from top-level
    assert_eq!(pull.envfile, Some(PathBuf::from("top.env"))); // Inherited from top-level
    assert_eq!(pull.shell, Some("top_shell".to_string())); // Inherited from top-level
    assert_eq!(pull.plugin, Some("top_plugin".to_string())); // Inherited from top-level
    assert_eq!(pull.script, Some("pull_script".to_string())); // Specific pull value

    let pull_vars = pull.vars.as_ref().unwrap();
    assert_eq!(pull_vars.get("k1").unwrap(), "v1_top"); // Inherited
    assert_eq!(pull_vars.get("k2").unwrap(), "v2_pull"); // Kept override

    let pull_requires = pull.requires.as_ref().unwrap();
    assert!(pull_requires.contains(&PathBuf::from("req_top"))); // Merged
    assert!(pull_requires.contains(&PathBuf::from("req_pull"))); // Merged

    // Verify push
    let push = resolved.push.as_ref().unwrap();
    assert_eq!(push.cwd, Some(true));
    assert_eq!(push.envfile, Some(PathBuf::from("top.env")));
    assert_eq!(push.shell, Some("top_shell".to_string()));
    assert_eq!(push.plugin, Some("top_plugin".to_string()));
    assert!(push.script.is_none());
}

#[test]
fn test_resolve_upstream_with_metadata_flags() -> anyhow::Result<()> {
    let temp_dir = assert_fs::TempDir::new()?;

    let cottage_toml_content = r#"
[upstream.origin]
cwd = true
envfile = "origin.env"

[upstream.origin.pull]
script = "pull.sh"

[upstream.origin.push]
script = "push.sh"
"#;

    let proj =
        Project::generate_test_project(temp_dir.path()).with_toml_config(cottage_toml_content)?;

    // Case 1: pull = true, push = true
    let meta_both = UpstreamMetadata {
        vars: None,
        pull: Some(true),
        push: Some(true),
    };
    let resolved = proj.resolve_upstream("origin", &meta_both).unwrap();
    assert!(resolved.pull.is_some());
    assert!(resolved.push.is_some());

    // Case 2: pull = true, push = false
    let meta_pull_only = UpstreamMetadata {
        vars: None,
        pull: Some(true),
        push: Some(false),
    };
    let resolved = proj.resolve_upstream("origin", &meta_pull_only).unwrap();
    assert!(resolved.pull.is_some());
    assert!(resolved.push.is_none());

    // Case 3: pull = false, push = true
    let meta_push_only = UpstreamMetadata {
        vars: None,
        pull: Some(false),
        push: Some(true),
    };
    let resolved = proj.resolve_upstream("origin", &meta_push_only).unwrap();
    assert!(resolved.pull.is_none());
    assert!(resolved.push.is_some());

    // Case 4: pull = false, push = false (or None)
    let meta_none = UpstreamMetadata {
        vars: None,
        pull: None,
        push: None,
    };
    let resolved = proj.resolve_upstream("origin", &meta_none).unwrap();
    assert!(resolved.pull.is_none());
    assert!(resolved.push.is_none());

    Ok(())
}

#[test]
fn test_resolve_upstream_comprehensive() -> anyhow::Result<()> {
    use indexmap::IndexMap;

    let temp_dir = assert_fs::TempDir::new()?;

    let cottage_toml_content = r#"
[upstream.defaults]
cwd = true
envfile = "default.env"
shell = "/bin/sh"
plugin = "default-plugin"

[upstream.defaults.vars]
a = "defaults_top"
b = "defaults_top"
c = "defaults_top"
d = "defaults_top"
e = "defaults_top"

[upstream.defaults.pull]
cwd = false
envfile = "default_pull.env"
shell = "/bin/bash"
plugin = "default-pull-plugin"
script = "default_pull_script.sh"
requires = ["req1.txt", "req2.txt"]

[upstream.defaults.pull.vars]
b = "defaults_pull"
c = "defaults_pull"
d = "defaults_pull"
e = "defaults_pull"

[upstream.defaults.push]
cwd = true
envfile = "default_push.env"
shell = "/bin/zsh"
plugin = "default-push-plugin"
script = "default_push_script.sh"

[upstream.origin]
cwd = true
envfile = "origin.env"
shell = "/usr/bin/fish"
plugin = "origin-plugin"

[upstream.origin.vars]
c = "upstream_top"
d = "upstream_top"
e = "upstream_top"

[upstream.origin.pull]
cwd = true
envfile = "origin_pull.env"
shell = "/bin/dash"
plugin = "origin-pull-plugin"
script = "origin_pull_script.sh"
requires = ["req3.txt", "req4.txt"]

[upstream.origin.pull.vars]
d = "upstream_pull"
e = "upstream_pull"
"#;

    let proj =
        Project::generate_test_project(temp_dir.path()).with_toml_config(cottage_toml_content)?;

    // Verify resolving nonexistent or reserved
    assert!(
        proj.resolve_upstream("nonexistent", &UpstreamMetadata::default())
            .is_none()
    );
    assert!(
        proj.resolve_upstream("defaults", &UpstreamMetadata::default())
            .is_none()
    );

    // Setup metadata vars (layer 1)
    let meta = UpstreamMetadata {
        vars: Some({
            let mut map = IndexMap::new();
            map.insert("e".to_string(), "meta".to_string());
            map
        }),
        pull: Some(true),
        push: Some(true),
    };

    let resolved = proj.resolve_upstream("origin", &meta).unwrap();
    let pull = resolved.pull.as_ref().unwrap();

    // Verify 5-layer variables priority:
    let pull_vars = pull.vars.as_ref().expect("vars should exist");
    assert_eq!(pull_vars.get("a").map(|v| v.as_str()), Some("defaults_top"));
    assert_eq!(
        pull_vars.get("b").map(|v| v.as_str()),
        Some("defaults_pull")
    );
    assert_eq!(pull_vars.get("c").map(|v| v.as_str()), Some("upstream_top"));
    assert_eq!(
        pull_vars.get("d").map(|v| v.as_str()),
        Some("upstream_pull")
    );
    assert_eq!(pull_vars.get("e").map(|v| v.as_str()), Some("meta"));

    // Verify pull properties
    assert_eq!(pull.cwd, Some(true));
    assert_eq!(pull.envfile, Some(PathBuf::from("origin_pull.env")));
    assert_eq!(pull.shell, Some("/bin/dash".to_string()));
    assert_eq!(pull.plugin, Some("origin-pull-plugin".to_string()));
    assert_eq!(pull.script, Some("origin_pull_script.sh".to_string()));

    // Verify requires accumulation/merging
    let pull_requires = pull.requires.as_ref().expect("requires should exist");
    assert!(pull_requires.contains(&PathBuf::from("req1.txt")));
    assert!(pull_requires.contains(&PathBuf::from("req2.txt")));
    assert!(pull_requires.contains(&PathBuf::from("req3.txt")));
    assert!(pull_requires.contains(&PathBuf::from("req4.txt")));
    assert_eq!(pull_requires.len(), 4);

    // Verify push (inherits origin top-level first, then defaults if absent)
    let push = resolved.push.as_ref().unwrap();
    assert_eq!(push.cwd, Some(true)); // from defaults.push (since origin has cwd=true as well)
    assert_eq!(push.envfile, Some(PathBuf::from("origin.env"))); // from origin top-level
    assert_eq!(push.shell, Some("/usr/bin/fish".to_string())); // from origin top-level
    assert_eq!(push.plugin, Some("origin-plugin".to_string())); // from origin top-level
    assert_eq!(push.script, Some("default_push_script.sh".to_string())); // from defaults.push

    Ok(())
}
