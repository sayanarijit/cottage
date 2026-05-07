use crate::{is_encrypted_path, to_decrypted_path};
use age::secrecy::ExposeSecret;
use anyhow::{Context, Result, anyhow};
use std::fs::OpenOptions;
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const COTTAGE_GITATTRIBUTES_LINE: &str =
    "*.cott.age binary export-ignore filter=cottage-encrypted -diff";

#[derive(Debug)]
pub struct Git {
    root_gitattributes: PathBuf,
}

impl Git {
    pub fn root_gitattributes(&self) -> &Path {
        &self.root_gitattributes
    }
}

#[derive(Debug)]
pub struct Project {
    cwd: PathBuf,
    root: PathBuf,
    global_identity_path: PathBuf,
    recipients_path: PathBuf,
    identity_path: PathBuf,
    ssh_dir: PathBuf,
    git: Option<Git>,
}

impl Project {
    pub fn init() -> Result<Self> {
        Self::load().or_else(|_| {
            log::debug!("project root not found, initializing new project");

            let cwd = std::env::current_dir().context("could not get current working directory")?;
            let root = get_root(&cwd, ".git/")
                .or_else(|| get_root(&cwd, ".jj/"))
                .unwrap_or(cwd);

            let cottage_dir = root.join(".cottage");
            std::fs::create_dir(&cottage_dir).with_context(|| {
                format!(
                    "{}: could not create cottage directory",
                    cottage_dir.display()
                )
            })?;
            log::debug!("{}: created directory", cottage_dir.display());
            Self::load().context("could not load project after initialization")
        })
    }

    pub fn load() -> Result<Self> {
        let cwd = std::env::current_dir().context("could not get current working directory")?;
        let root = get_project_root(&cwd).context(format!(
            "{}: could not find project root (looking for .cottage/, .git/, or .jj/)",
            cwd.display()
        ))?;
        log::debug!("{}: project root identified", root.display());

        let cottage_dir = root.join(".cottage");
        if !cottage_dir.exists() {
            std::fs::create_dir(&cottage_dir).with_context(|| {
                format!(
                    "{}: could not create cottage directory",
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
                    "{}: could not create recipients directory",
                    recipients_path.display()
                )
            })?;
            log::debug!("{}: created directory", recipients_path.display());
            std::fs::write(&recipient_path, pk.to_string()).with_context(|| {
                format!(
                    "{}: could not write recipient file",
                    recipient_path.display()
                )
            })?;
            log::debug!("{}: wrote file", recipient_path.display());
            std::fs::write(&identity_path, sk.to_string().expose_secret()).with_context(|| {
                format!("{}: could not write identity file", identity_path.display())
            })?;
            log::debug!("{}: wrote file", identity_path.display());
        };

        let git = if root.join(".git").exists() {
            Some(Git {
                root_gitattributes: root.join(".gitattributes"),
            })
        } else {
            None
        };

        if let Some(git) = &git {
            append_to_gitignore_if_absent(&identity_path, false)?;
            append_line_if_absent(git.root_gitattributes(), COTTAGE_GITATTRIBUTES_LINE, false)?;
        }

        let global_config_dir = dirs::home_dir()
            .map(|h| h.join(".config/cottage"))
            .context("could not determine age config directory")?;

        let global_identity_path = global_config_dir.join("identity");

        let ssh_dir = dirs::home_dir()
            .map(|h| h.join(".ssh"))
            .context("could not determine ssh directory")?;

        Ok(Self {
            cwd,
            root,
            recipients_path,
            identity_path,
            git,
            ssh_dir,
            global_identity_path,
        })
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn recipients_path(&self) -> &Path {
        &self.recipients_path
    }

    pub fn identity_path(&self) -> &Path {
        &self.identity_path
    }

    pub fn ssh_dir(&self) -> &Path {
        &self.ssh_dir
    }

    pub fn git(&self) -> Option<&Git> {
        self.git.as_ref()
    }

    pub fn relative_to_cwd(&self, path: &Path) -> PathBuf {
        pathdiff::diff_paths(path, self.cwd()).unwrap_or_else(|| path.to_path_buf())
    }

    pub fn relative_to_root(&self, path: &Path) -> PathBuf {
        pathdiff::diff_paths(path, self.root()).unwrap_or_else(|| path.to_path_buf())
    }

    pub fn global_identity_path(&self) -> &PathBuf {
        &self.global_identity_path
    }

    pub fn clean(&self, dry_run: bool) -> Result<()> {
        if self.root().join(".cottage").exists() {
            if dry_run {
                log::debug!(
                    "{}: would remove directory (dry run)",
                    self.root().join(".cottage").display()
                );
            } else {
                std::fs::remove_dir_all(self.root().join(".cottage")).with_context(|| {
                    format!(
                        "{}: could not remove .cottage directory",
                        self.root().display()
                    )
                })?;
                log::debug!(
                    "{}: removed directory",
                    self.root().join(".cottage").display()
                );
            }
        }

        if let Some(git) = self.git() {
            remove_line_if_present(
                git.root_gitattributes(),
                COTTAGE_GITATTRIBUTES_LINE,
                dry_run,
            )?;
            remove_from_gitignore_if_present(self.identity_path(), dry_run)?;
        }
        Ok(())
    }
}

pub fn iter_encrypted(path: &Path) -> impl Iterator<Item = walkdir::DirEntry> {
    walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| is_encrypted_path(e.path()))
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
}

pub fn append_line_if_absent(path: &Path, line: &str, dry_run: bool) -> Result<bool> {
    let line = line.trim();
    log::trace!("{}: checking if line {:?} is present", path.display(), line);
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .with_context(|| format!("{}: could not open", path.display()))?;

    if std::io::BufReader::new(&file)
        .lines()
        .map_while(Result::ok)
        .any(|l| l.trim() == line)
    {
        log::trace!("{}: line {:?} already present", path.display(), line);
        return Ok(false);
    }

    if dry_run {
        log::trace!(
            "{}: line {:?} would be added (dry run)",
            path.display(),
            line
        );
        return Ok(true);
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
        writeln!(file).with_context(|| format!("{}: could not write", path.display()))?;
    }

    writeln!(file, "{}", line).with_context(|| format!("{}: could not write", path.display()))?;
    Ok(true)
}

pub fn remove_line_if_present(path: &Path, line: &str, dry_run: bool) -> Result<bool> {
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

    if dry_run {
        log::trace!(
            "{}: line {:?} would be removed (dry run)",
            path.display(),
            line
        );
    } else {
        log::trace!("{}: removing line {:?} from file", path.display(), line);
        let lines: Vec<String> = std::io::BufReader::new(std::fs::File::open(path)?)
            .lines()
            .map_while(Result::ok)
            .filter(|l| l.trim() != line)
            .collect();

        std::fs::write(path, lines.join("\n") + "\n")?;
    }

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
            "{}: could not get decrypted path for encrypted file",
            path.display()
        ))?
    }

    Ok(PathBuf::from("/")
        .join(pathdiff::diff_paths(&abs_path, &abs_root).context(format!(
            "{}: could not get relative path for gitignore",
            path.display()
        ))?)
        .to_string_lossy()
        .to_string())
}

// Very naive implementation for now
pub fn append_to_gitignore_if_absent(path: &Path, dry_run: bool) -> Result<Option<PathBuf>> {
    let gitignore_root = get_or_create_gitignore_root(path)?;
    let line_to_add = fmt_gitignore_line(path, &gitignore_root)?;

    let gitignore_path = gitignore_root.join(".gitignore");
    if append_line_if_absent(&gitignore_path, &line_to_add, dry_run)? {
        log::debug!("{}: added to {}", line_to_add, gitignore_path.display());
        Ok(Some(gitignore_path))
    } else {
        Ok(None)
    }
}

// Very naive implementation for now
pub fn remove_from_gitignore_if_present(path: &Path, dry_run: bool) -> Result<Option<PathBuf>> {
    let gitignore_root = get_or_create_gitignore_root(path)?;
    let line_to_remove = fmt_gitignore_line(path, &gitignore_root)?;

    let gitignore_path = gitignore_root.join(".gitignore");
    if remove_line_if_present(&gitignore_path, &line_to_remove, dry_run)? {
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

#[cfg(test)]
impl Project {
    pub fn generate_test_project(path: &Path) -> Self {
        let root = path.to_path_buf();
        let cottage_dir = root.join(".cottage");
        std::fs::create_dir_all(&cottage_dir).unwrap();
        let recipients_path = cottage_dir.join("recipients");
        let identity_path = cottage_dir.join("identity");
        let global_config_dir = root.join(".config/cottage");
        let global_identity_path = global_config_dir.join("identity");
        let ssh_dir = root.join(".ssh");

        Self {
            cwd: root.clone(),
            root,
            recipients_path,
            identity_path,
            git: None,
            ssh_dir,
            global_identity_path,
        }
    }

    pub fn init_test_recipients(&self) {
        let sk = age::x25519::Identity::generate();
        let pk = sk.to_public();
        std::fs::create_dir_all(&self.recipients_path).unwrap();
        std::fs::write(self.recipients_path.join("test"), pk.to_string()).unwrap();
        std::fs::write(&self.identity_path, sk.to_string().expose_secret()).unwrap();
    }

    pub fn load_test_identities(&self) -> Box<dyn Iterator<Item = crate::Identity>> {
        crate::identity::load_identities(self, vec![])
    }

    pub fn load_test_recipients(&self) -> Box<dyn Iterator<Item = crate::RecipientData> + '_> {
        crate::recipients::load_recipients(self, vec![], None)
    }
}
