use age::secrecy::ExposeSecret;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug)]
pub struct Git {
    root_gitignore: PathBuf,
    root_gitattributes: PathBuf,
}

impl Git {
    pub fn root_gitignore(&self) -> &Path {
        &self.root_gitignore
    }

    pub fn root_gitattributes(&self) -> &Path {
        &self.root_gitattributes
    }
}

#[derive(Debug)]
pub struct Project {
    cwd: PathBuf,
    root: PathBuf,
    cottage_dir: PathBuf,
    recipients_path: PathBuf,
    identity_path: PathBuf,
    git: Option<Git>,
}

impl Project {
    pub fn init() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current working directory")?;
        let root = get_project_root(&cwd)
            .context("Could not find project root (looking for .cottage/ or .git/)")?;

        let cottage_dir = root.join(".cottage");
        if !cottage_dir.exists() {
            std::fs::create_dir(&cottage_dir).with_context(|| {
                format!("Failed to create cottage directory at {:?}", cottage_dir)
            })?;
        }
        let recipients_path = cottage_dir.join("recipients");
        let identity_path = cottage_dir.join("identity");
        if !identity_path.exists() && !recipients_path.exists() {
            let reicipient = whoami::username().unwrap_or_else(|_| {
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string()
            });
            let recipient_path = recipients_path.join(&reicipient);

            let sk = age::x25519::Identity::generate();
            let pk = sk.to_public();

            std::fs::create_dir_all(&recipients_path).with_context(|| {
                format!(
                    "Failed to create recipient directory at {:?}",
                    recipient_path.parent().unwrap()
                )
            })?;
            std::fs::write(&recipient_path, pk.to_string()).with_context(|| {
                format!("Failed to write recipient file at {:?}", recipient_path)
            })?;
            std::fs::write(&identity_path, sk.to_string().expose_secret())
                .with_context(|| format!("Failed to write identity file at {:?}", identity_path))?;
        };

        let git = if root.join(".git").exists() {
            Some(Git {
                root_gitignore: root.join(".gitignore"),
                root_gitattributes: root.join(".gitattributes"),
            })
        } else {
            None
        };

        if let Some(git) = &git {
            append_to_gitignore_if_absent(&identity_path)?;
            append_line_if_absent(
                git.root_gitattributes(),
                "*.cott.age binary filter=cottage-encrypted",
            )?;
        }

        Ok(Self {
            cwd,
            root,
            cottage_dir,
            recipients_path,
            identity_path,
            git,
        })
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn cottage_dir(&self) -> &Path {
        &self.cottage_dir
    }

    pub fn recipients_path(&self) -> &Path {
        &self.recipients_path
    }

    pub fn identity_path(&self) -> &Path {
        &self.identity_path
    }

    pub fn git(&self) -> Option<&Git> {
        self.git.as_ref()
    }
}

fn get_root(cwd: &Path, root_identifier: &str) -> Option<PathBuf> {
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

fn get_project_root(cwd: &Path) -> Option<PathBuf> {
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

fn append_line_if_absent(path: &Path, line: &str) -> Result<bool> {
    let line = line.trim();
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open {:?}", path))?;

    if std::io::BufReader::new(&file)
        .lines()
        .filter_map(Result::ok)
        .any(|l| l.trim() == line)
    {
        return Ok(false);
    }

    let needs_nl = if file.seek(SeekFrom::End(0))? > 0 {
        let mut buf = [0; 1];
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut buf)?;
        file.seek(SeekFrom::End(0))?;
        buf[0] != b'\n'
    } else {
        false
    };

    if needs_nl {
        writeln!(file).with_context(|| format!("Failed to write newline to {:?}", path))?;
    }

    writeln!(file, "{}", line).with_context(|| format!("Failed to write to {:?}", path))?;
    Ok(true)
}

// Very naive implementation for now
pub(crate) fn append_to_gitignore_if_absent(path: &Path) -> Result<Option<PathBuf>> {
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
    if append_line_if_absent(&gitignore_path, &line_to_add)? {
        Ok(Some(gitignore_path))
    } else {
        Ok(None)
    }
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
        let added_path = append_to_gitignore_if_absent(&parent_secret)
            .unwrap()
            .unwrap();
        assert_eq!(added_path, parent_gitignore);

        let parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
        assert!(parent_content.contains("/secret.txt"));

        // Test adding to parent .gitignore when subdir .gitignore doesn't exist
        let added_subpath = append_to_gitignore_if_absent(&subdir_secret)
            .unwrap()
            .unwrap();
        assert_eq!(added_subpath, parent_gitignore);

        let parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
        assert!(parent_content.contains("/subdir/subsecret.txt"));

        // Test adding to subdir .gitignore
        std::fs::write(&subdir_gitignore, "").unwrap();
        let added_subpath_to_subdir = append_to_gitignore_if_absent(&subdir_secret).unwrap();

        assert_eq!(added_subpath_to_subdir, Some(subdir_gitignore.clone()));

        let subdir_content = std::fs::read_to_string(&subdir_gitignore).unwrap();
        assert!(subdir_content.contains("/subsecret.txt"));

        // Check duplicates are not added
        let duplicate_parent_add = append_to_gitignore_if_absent(&parent_secret).unwrap();
        let duplicate_subdir_add = append_to_gitignore_if_absent(&subdir_secret).unwrap();
        let updated_parent_content = std::fs::read_to_string(&parent_gitignore).unwrap();
        let updated_subdir_content = std::fs::read_to_string(&subdir_gitignore).unwrap();

        assert_eq!(duplicate_parent_add, None);
        assert_eq!(duplicate_subdir_add, None);

        assert_eq!(parent_content, updated_parent_content);
        assert_eq!(subdir_content, updated_subdir_content);
    }

    #[test]
    fn test_add_line_if_absent() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Test adding a line to an empty file
        append_line_if_absent(path, "line1").unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "line1\n");

        // Test adding the same line again (should not be added)
        append_line_if_absent(path, "line1").unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "line1\n");

        // Test adding a different line
        append_line_if_absent(path, "line2").unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "line1\nline2\n");

        // Test newline is added if file doesn't end with newline
        std::fs::write(path, "line1").unwrap();
        append_line_if_absent(path, "line2").unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "line1\nline2\n");
    }
}
