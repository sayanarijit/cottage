use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

pub fn get_root(cwd: &Path, root_identifier: &str) -> Option<PathBuf> {
    let start = std::path::absolute(cwd).ok()?;
    let mut current = start;
    while let Some(path) = current.parent() {
        if current.join(root_identifier).exists() {
            match (
                root_identifier.ends_with("/"),
                current.join(root_identifier).is_dir(),
            ) {
                (true, true) | (false, false) => return Some(current.to_path_buf()),
                _ => {}
            }
        }
        current = path.to_path_buf();
    }
    None
}

pub fn get_project_root(cwd: &Path) -> Option<PathBuf> {
    get_root(cwd, ".cottage/").or_else(|| get_root(cwd, ".git/"))
}

pub fn to_encrypted_path(path: &Path) -> PathBuf {
    path.with_added_extension("cott")
        .with_added_extension("age")
}

pub fn is_encrypted_path(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".cott.age")
}

pub fn to_decrypted_path(path: &Path) -> Option<PathBuf> {
    if is_encrypted_path(path) {
        path.file_stem()
            .and_then(|s| PathBuf::from(s).file_stem().map(|s| path.with_file_name(s)))
    } else {
        None
    }
}

// Very naive implementation for now
pub(crate) fn add_to_gitignore(path: &Path) -> Result<Option<PathBuf>> {
    let gitignote_root = get_root(path, ".gitignore")
        .or_else(|| get_root(path, ".git/"))
        .context("Could not find .gitignore or .git directory")?;

    let abs_root = std::path::absolute(&gitignote_root)?;
    let mut abs_path = std::path::absolute(&path)?;
    if is_encrypted_path(&abs_path) {
        abs_path = to_decrypted_path(&abs_path)
            .context("Failed to get decrypted path for encrypted file")?
    }

    let line_to_add = PathBuf::from("/")
        .join(&abs_path.strip_prefix(&abs_root)?)
        .to_string_lossy()
        .to_string();

    let gitignore_path = gitignote_root.join(".gitignore");

    if !gitignore_path.exists() {
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&gitignore_path)
            .with_context(|| format!("Failed to create {:?}", gitignore_path))?;
    }

    let mut content = std::fs::read_to_string(&gitignore_path)
        .with_context(|| format!("Failed to read {:?}", gitignore_path))?;
    if content.contains(&line_to_add) {
        return Ok(None);
    }

    let start = "# cottage managed secrets: start";
    let end = "# cottage managed secrets: end";

    match (content.find(start), content.find(end)) {
        (Some(start_idx), Some(mut end_idx)) => {
            if start_idx > end_idx {
                // Try to fix
                content = content
                    .lines()
                    .map(|l| {
                        if l == start {
                            end.to_string()
                        } else if l == end {
                            start.to_string()
                        } else {
                            l.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
            };
            end_idx = content
                .find(end)
                .context("Failed to find end marker after fixing")?;

            if content.get(end_idx.saturating_sub(1)..end_idx) == Some("\n") {
                content.insert_str(end_idx, &format!("{line_to_add}\n"));
            } else {
                content.insert_str(end_idx, &format!("\n{line_to_add}\n"));
            }
        }
        (Some(start_idx), None) => {
            content.insert_str(
                start_idx + start.chars().count(),
                &format!("\n{line_to_add}\n{end}\n\n"),
            );
        }
        (None, Some(end_idx)) => {
            content.insert_str(end_idx, &format!("\n\n{start}\n{line_to_add}\n"));
        }
        (None, None) => {
            content.push_str(&format!("\n\n{start}\n{line_to_add}\n{end}\n\n"));
        }
    };

    std::fs::write(&gitignore_path, content)
        .with_context(|| format!("Failed to write to {:?}", gitignore_path))?;

    Ok(Some(gitignore_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_root() {
        let root = tempfile::tempdir().unwrap();
        let subdir = root.path().join("subdir");

        std::fs::create_dir_all(&subdir.join(".cottage")).unwrap();

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
        let parent_dir = tempfile::tempdir().unwrap();
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
        let added_path = add_to_gitignore(&parent_secret).unwrap().unwrap();
        assert_eq!(added_path, parent_gitignore);

        let parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
        assert!(parent_content.contains("/secret.txt"));

        // Test adding to parent .gitignore when subdir .gitignore doesn't exist
        let added_subpath = add_to_gitignore(&subdir_secret).unwrap().unwrap();
        assert_eq!(added_subpath, parent_gitignore);

        let parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
        assert!(parent_content.contains("/subdir/subsecret.txt"));

        // Test adding to subdir .gitignore
        std::fs::write(&subdir_gitignore, "").unwrap();
        let added_subpath_to_subdir = add_to_gitignore(&subdir_secret).unwrap();

        assert_eq!(added_subpath_to_subdir, Some(subdir_gitignore.clone()));

        let subdir_content = std::fs::read_to_string(&subdir_gitignore).unwrap();
        assert!(subdir_content.contains("/subsecret.txt"));

        // Check duplicates are not added
        let duplicate_parent_add = add_to_gitignore(&parent_secret).unwrap();
        let duplicate_subdir_add = add_to_gitignore(&subdir_secret).unwrap();
        let updated_parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
        let updated_subdir_content = std::fs::read_to_string(&subdir_gitignore).unwrap();

        assert_eq!(duplicate_parent_add, None);
        assert_eq!(duplicate_subdir_add, None);

        assert_eq!(parent_content, updated_parent_content);
        assert_eq!(subdir_content, updated_subdir_content);
    }
}
