use std::path::{Path, PathBuf};

pub fn get_root(cwd: &Path, root_identifier: &str) -> Option<PathBuf> {
    let start = cwd.canonicalize().ok()?;
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
