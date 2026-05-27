use crate::project::ResolvedUpstream;
use crate::{DecryptOptions, Identity, Project, decrypt_into_cmd};
use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::{Context, Result};
use colored::Colorize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Encrypt,
    Decrypt,
    Delete,
    Pull,
    Push,
}

impl std::fmt::Display for OperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationKind::Encrypt => write!(f, "{}", "encrypt".green()),
            OperationKind::Decrypt => write!(f, "{}", "decrypt".cyan()),
            OperationKind::Delete => write!(f, "{}", "delete ".red()),
            OperationKind::Pull => write!(f, "{}", "pull   ".magenta()),
            OperationKind::Push => write!(f, "{}", "push   ".magenta()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub kind: OperationKind,
    pub input: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationResult {
    pub kind: OperationKind,
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub edits: Vec<PathBuf>,
    pub cleanups: Vec<PathBuf>,
}

impl From<Operation> for OperationResult {
    fn from(op: Operation) -> Self {
        Self {
            kind: op.kind,
            input: op.input,
            output: Some(op.output),
            edits: vec![],
            cleanups: vec![],
        }
    }
}

pub fn print_result(
    mut file: impl Write,
    proj: &Project,
    op: &OperationResult,
    compact: bool,
) -> Result<()> {
    match (op.kind, compact, op.output.as_ref()) {
        (kind, false, Some(output)) => {
            writeln!(
                file,
                "{} {}\n   {} {}",
                kind,
                proj.relative_to_cwd(&op.input).display(),
                "into".blue(),
                proj.relative_to_cwd(output).display()
            )?;
            print_cleanups(&mut file, proj, &op.cleanups)?;
            print_edits(&mut file, proj, &op.edits)?;
        }
        (kind, false, None) => {
            writeln!(
                file,
                "{} {}",
                kind,
                proj.relative_to_cwd(&op.input).display()
            )?;
            print_cleanups(&mut file, proj, &op.cleanups)?;
            print_edits(&mut file, proj, &op.edits)?;
        }
        (_, true, _) => {
            writeln!(
                file,
                "{}",
                proj.relative_to_cwd(&op.input)
                    .display()
                    .to_string()
                    .green()
            )?;
        }
    }
    Ok(())
}

fn print_edits(mut file: impl Write, proj: &Project, edits: &[PathBuf]) -> Result<()> {
    for path in edits {
        writeln!(
            file,
            "   {} {}",
            "edit".yellow(),
            proj.relative_to_cwd(path).display()
        )?;
    }
    Ok(())
}

fn print_cleanups(mut file: impl Write, proj: &Project, cleanups: &[PathBuf]) -> Result<()> {
    for path in cleanups {
        writeln!(
            file,
            "{} {}",
            " delete".red(),
            proj.relative_to_cwd(path).display()
        )?;
    }
    Ok(())
}

pub fn to_encrypted_path(path: &Path) -> PathBuf {
    path.with_added_extension("cott")
        .with_added_extension("age")
}

pub fn to_metadata_path(path: &Path) -> PathBuf {
    path.with_added_extension("cott")
        .with_added_extension("toml")
}

pub fn is_encrypted_path(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".cott.age")
}

pub fn is_metadata_path(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".cott.toml")
}

pub fn to_decrypted_path(path: &Path) -> Option<PathBuf> {
    if is_encrypted_path(path) || is_metadata_path(path) {
        path.file_stem()
            .and_then(|s| PathBuf::from(s).file_stem().map(|s| path.with_file_name(s)))
    } else {
        None
    }
}

/// Securely removes a file by overwriting it with zeros and syncing to disk before unlinking.
pub fn secure_remove_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let metadata = path.symlink_metadata().with_context(|| {
        format!("{}: failed to get metadata for secure removal", path.display())
    })?;

    if metadata.is_file() {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .with_context(|| format!("{}: failed to open for secure removal", path.display()))?;

        let length = metadata.len();
        if length > 0 {
            let chunk_size = 65536;
            let mut remaining = length;
            let chunk = vec![0u8; std::cmp::min(remaining as usize, chunk_size)];
            while remaining > 0 {
                let to_write = std::cmp::min(remaining, chunk_size as u64);
                file.write_all(&chunk[..to_write as usize])?;
                remaining -= to_write;
            }
            file.sync_all()?;
        }
    }

    std::fs::remove_file(path)
        .with_context(|| format!("{}: failed to remove file", path.display()))?;
    Ok(())
}

pub(crate) fn run_upstream_script(
    proj: &Project,
    identities: &[Identity],
    metadata_path: &Path,
    upstream_name: &str,
    upstream: &ResolvedUpstream,
    kind: OperationKind,
    stdin: Option<SecretSlice<u8>>,
    debug: bool,
) -> Result<SecretSlice<u8>> {
    let secretdir = metadata_path
        .parent()
        .context("metadata file has no parent directory")?;

    let (cfg, op) = match (kind, upstream.pull.as_ref(), upstream.push.as_ref()) {
        (OperationKind::Pull, Some(cfg), _) => (cfg, "pull"),
        (OperationKind::Push, _, Some(cfg)) => (cfg, "push"),
        (OperationKind::Pull, None, _) => {
            return Err(anyhow::anyhow!(
                "{upstream_name}: pull operation is not configured for this upstream",
            ));
        }
        (OperationKind::Push, _, None) => {
            return Err(anyhow::anyhow!(
                "{upstream_name}: push operation is not configured for this upstream",
            ));
        }
        _ => {
            anyhow::bail!("{upstream_name}: unsupported operation for this upstream")
        }
    };

    let (mut cmd, _tmppath) = match (cfg.plugin.as_ref(), cfg.script.as_ref()) {
        (Some(plugin), None) => {
            let mut cmd = Command::new(plugin);
            cmd.arg(op);
            (cmd, None)
        }
        (None, Some(script)) => {
            let tmpfile = tempfile::Builder::new()
                .prefix(".ctg-upstream-")
                .suffix(".sh")
                .tempfile_in(secretdir)
                .with_context(|| {
                    format!(
                        "{}: failed to create temporary file for upstream script",
                        upstream_name
                    )
                })?
                .into_temp_path();

            std::fs::write(&tmpfile, script).with_context(|| {
                format!(
                    "{}: failed to write upstream script to temporary file",
                    upstream_name
                )
            })?;

            let mut cmd = Command::new(cfg.shell.as_deref().unwrap_or("sh"));
            cmd.arg(tmpfile.as_os_str());
            (cmd, Some(tmpfile))
        }
        _ => {
            anyhow::bail!(
                "{upstream_name}: upstream script must specify either a plugin or a script, but not both"
            )
        }
    };

    if let Some(envfile) = &cfg.envfile {
        let dec_opts = DecryptOptions {
            identities: identities.to_vec(),
            recipients: vec![],
            dry_run: true,
            skip_gitignore: true,
            skip_timestamps: true,
            skip_verify_encrypted: true,
            skip_verify_recipients: true,
        };
        let envfile = if envfile.is_relative() {
            proj.root().join(envfile)
        } else {
            envfile.to_path_buf()
        };

        decrypt_into_cmd(proj, &mut cmd, &envfile, &dec_opts)?;
    }

    if let Some(vars) = &cfg.vars {
        for (k, v) in vars {
            cmd.env(k, v);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        cmd.env("ctg", exe.canonicalize()?);
    }

    cmd.stdout(Stdio::piped());
    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    }
    if !debug {
        cmd.stderr(Stdio::null());
    }

    if let Some(true) = &cfg.cwd {
        cmd.current_dir(secretdir);
    }

    let mut proc = cmd.spawn().context("failed to spawn upstream script")?;
    if let Some(stdin) = stdin {
        if let Some(mut procstdin) = proc.stdin.take() {
            procstdin
                .write_all(stdin.expose_secret())
                .with_context(|| {
                    format!("{upstream_name}: failed to write to upstream script stdin")
                })?;
        } else {
            anyhow::bail!("{upstream_name}: upstream script does not have a stdin to write to");
        }
    }

    let output = proc.wait_with_output().with_context(|| {
        format!("{upstream_name}: failed to wait for upstream script to finish")
    })?;

    if !output.status.success() {
        anyhow::bail!(
            "{upstream_name}: upstream script exited with non-zero status code {:?}",
            output.status.code()
        );
    }

    Ok(SecretSlice::new(output.stdout.into()))
}
