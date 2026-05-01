use crate::{is_encrypted_path, to_decrypted_path};
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
            .context("Could not find project root (looking for any of .cottage/, .git/ or .jj/)")?;

        let cottage_dir = root.join(".cottage");
        if !cottage_dir.exists() {
            std::fs::create_dir(&cottage_dir).with_context(|| {
                format!(
                    "{}: failed to create cottage directory",
                    cottage_dir.display()
                )
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
                    "{}: failed to create recipients directory",
                    recipients_path.display()
                )
            })?;
            std::fs::write(&recipient_path, pk.to_string()).with_context(|| {
                format!(
                    "{}: failed to write recipient file",
                    recipient_path.display()
                )
            })?;
            std::fs::write(&identity_path, sk.to_string().expose_secret()).with_context(|| {
                format!("{}: failed to write identity file", identity_path.display())
            })?;
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

    pub fn relative_to_cwd(&self, path: &Path) -> PathBuf {
        pathdiff::diff_paths(path, &self.cwd).unwrap_or_else(|| path.to_path_buf())
    }

    pub fn relative_to_root(&self, path: &Path) -> PathBuf {
        pathdiff::diff_paths(path, &self.root).unwrap_or_else(|| path.to_path_buf())
    }
}

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
    get_root(cwd, ".cottage/")
        .or_else(|| get_root(cwd, ".git/"))
        .or_else(|| get_root(cwd, ".jj/"))
}

pub fn append_line_if_absent(path: &Path, line: &str) -> Result<bool> {
    let line = line.trim();
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .with_context(|| format!("{}: failed to open", path.display()))?;

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
        writeln!(file).with_context(|| format!("{}: failed to write", path.display()))?;
    }

    writeln!(file, "{}", line).with_context(|| format!("{}: failed to write", path.display()))?;
    Ok(true)
}

// Very naive implementation for now
pub fn append_to_gitignore_if_absent(path: &Path) -> Result<Option<PathBuf>> {
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
        .join(
            pathdiff::diff_paths(&abs_path, &abs_root)
                .context("Failed to get relative path for gitignore")?,
        )
        .to_string_lossy()
        .to_string();

    let gitignore_path = gitignote_root.join(".gitignore");
    if append_line_if_absent(&gitignore_path, &line_to_add)? {
        Ok(Some(gitignore_path))
    } else {
        Ok(None)
    }
}
