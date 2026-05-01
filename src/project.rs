use crate::{is_encrypted_path, to_decrypted_path};
use age::secrecy::ExposeSecret;
use anyhow::{Context, Result, anyhow};
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
        let cwd = std::env::current_dir().context("failed to get current working directory")?;
        let root = get_project_root(&cwd).context(format!(
            "{}: failed to find project root (looking for .cottage/, .git/, or .jj/)",
            cwd.display()
        ))?;
        log::debug!("{}: project root identified", root.display());

        let cottage_dir = root.join(".cottage");
        if !cottage_dir.exists() {
            std::fs::create_dir(&cottage_dir).with_context(|| {
                format!(
                    "{}: failed to create cottage directory",
                    cottage_dir.display()
                )
            })?;
            log::debug!("{}: created directory", cottage_dir.display());
        }
        let recipients_path = cottage_dir.join("recipients");
        let identity_path = cottage_dir.join("identity");
        if !identity_path.exists() && !recipients_path.exists() {
            let recipient = whoami::username().unwrap_or_else(|_| {
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string()
            });
            let recipient_path = recipients_path.join(&recipient);
            log::debug!(
                "init: {}: creating new recipient and identity for user {}",
                recipient_path.display(),
                recipient
            );

            let sk = age::x25519::Identity::generate();
            let pk = sk.to_public();

            std::fs::create_dir_all(&recipients_path).with_context(|| {
                format!(
                    "{}: failed to create recipients directory",
                    recipients_path.display()
                )
            })?;
            log::debug!("{}: created directory", recipients_path.display());
            std::fs::write(&recipient_path, pk.to_string()).with_context(|| {
                format!(
                    "{}: failed to write recipient file",
                    recipient_path.display()
                )
            })?;
            log::debug!("{}: wrote file", recipient_path.display());
            std::fs::write(&identity_path, sk.to_string().expose_secret()).with_context(|| {
                format!("{}: failed to write identity file", identity_path.display())
            })?;
            log::debug!("{}: wrote file", identity_path.display());
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
    log::trace!("{}: checking if line {:?} is present", path.display(), line);
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .with_context(|| format!("{}: failed to open", path.display()))?;

    if std::io::BufReader::new(&file)
        .lines()
        .map_while(Result::ok)
        .any(|l| l.trim() == line)
    {
        log::trace!("{}: line {:?} already present", path.display(), line);
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

pub fn remove_line_if_present(path: &Path, line: &str) -> Result<bool> {
    let line = line.trim();
    log::trace!(
        "{}: checking if line {:?} is present for removal",
        path.display(),
        line
    );
    if !path.exists() {
        return Ok(false);
    }

    if !std::io::BufReader::new(std::fs::File::open(path)?)
        .lines()
        .map_while(Result::ok)
        .any(|l| l.trim() == line)
    {
        log::trace!("{}: line {:?} not found", path.display(), line);
        return Ok(false);
    }

    let lines: Vec<String> = std::io::BufReader::new(std::fs::File::open(path)?)
        .lines()
        .map_while(Result::ok)
        .filter(|l| l.trim() != line)
        .collect();

    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(true)
}

fn get_or_create_gitignore_root(path: &Path) -> Result<PathBuf> {
    if let Some(dir) = get_root(path, ".gitignore").or_else(|| get_root(path, ".git/")) {
        let gi = dir.join(".gitignore");
        if !gi.exists() {
            std::fs::write(&gi, "")?;
        }
        Ok(dir)
    } else {
        Err(anyhow!(
            "{}: could not find .gitignore or .git/ parent",
            path.display(),
        ))
    }
}

pub fn fmt_gitignore_line(path: &Path, gitignore_root: &Path) -> Result<String> {
    let abs_root = std::path::absolute(gitignore_root)?;
    let mut abs_path = std::path::absolute(path)?;
    if is_encrypted_path(&abs_path) {
        abs_path = to_decrypted_path(&abs_path).context(format!(
            "{}: failed to get decrypted path for encrypted file",
            path.display()
        ))?
    }

    Ok(PathBuf::from("/")
        .join(pathdiff::diff_paths(&abs_path, &abs_root).context(format!(
            "{}: failed to get relative path for gitignore",
            path.display()
        ))?)
        .to_string_lossy()
        .to_string())
}

// Very naive implementation for now
pub fn append_to_gitignore_if_absent(path: &Path) -> Result<Option<PathBuf>> {
    let gitignote_root = get_or_create_gitignore_root(path)?;
    let line_to_add = fmt_gitignore_line(path, &gitignote_root)?;

    let gitignore_path = gitignote_root.join(".gitignore");
    if append_line_if_absent(&gitignore_path, &line_to_add)? {
        log::debug!("{}: added to {}", line_to_add, gitignore_path.display());
        Ok(Some(gitignore_path))
    } else {
        Ok(None)
    }
}

// Very naive implementation for now
pub fn remove_from_gitignore_if_present(path: PathBuf) -> Result<Option<PathBuf>> {
    let gitignote_root = get_or_create_gitignore_root(&path)?;
    let line_to_remove = fmt_gitignore_line(&path, &gitignote_root)?;

    let gitignore_path = gitignote_root.join(".gitignore");
    if remove_line_if_present(&gitignore_path, &line_to_remove)? {
        log::debug!(
            "{}: removed from {}",
            line_to_remove,
            gitignore_path.display()
        );
        Ok(Some(gitignore_path))
    } else {
        Ok(None)
    }
}
