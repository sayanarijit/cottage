use crate::project::ResolvedUpstream;
use crate::status::status_file;
use crate::{
    CleanOptions, DecryptOptions, Identity, Project, RecipientData, StatusOptions, decrypt_file,
    decrypt_into_cmd,
};
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
        format!(
            "{}: failed to get metadata for secure removal",
            path.display()
        )
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
            let plugin_path = Path::new(plugin);
            let resolved_plugin = if plugin_path.is_relative() && plugin_path.components().count() > 1 {
                proj.root().join(plugin_path)
            } else {
                plugin_path.to_path_buf()
            };
            let mut cmd = Command::new(resolved_plugin);
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

#[derive(Debug)]
pub struct TempDecryptedFile {
    path: PathBuf,
    was_present: bool,
    force_cleanup: bool,
    disarmed: bool,
}

impl TempDecryptedFile {
    pub fn new(path: PathBuf, force_cleanup: bool) -> Self {
        let was_present = path.exists();
        Self {
            path,
            was_present,
            force_cleanup,
            disarmed: false,
        }
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn was_present(&self) -> bool {
        self.was_present
    }

    pub fn disarm(&mut self) {
        self.disarmed = true;
    }

    pub fn cleanup(&mut self, dry_run: bool) -> Result<Option<OperationResult>> {
        self.disarm();
        if self.force_cleanup || !self.was_present {
            let clean_opts = CleanOptions {
                dry_run,
                gitignore: false,
                encrypted: false,
            };
            crate::clean::clean_file(self.path.clone(), &clean_opts)
        } else {
            Ok(None)
        }
    }
}

impl Drop for TempDecryptedFile {
    fn drop(&mut self) {
        if !self.disarmed
            && (self.force_cleanup || !self.was_present)
            && self.path.exists()
            && let Err(e) = secure_remove_file(&self.path)
        {
            log::error!(
                "failed to secure remove temporary decrypted file {}: {:?}",
                self.path.display(),
                e
            );
        }
    }
}

pub(crate) fn decrypt_required_secrets(
    proj: &Project,
    requires: Option<&indexmap::IndexSet<PathBuf>>,
    vars: Option<&indexmap::IndexMap<String, String>>,
    identities: &[Identity],
    recipients: &[RecipientData],
    upstream_name: &str,
    skip_gitignore: bool,
) -> Result<Vec<TempDecryptedFile>> {
    let mut req_decrypted = vec![];
    let mut requires: indexmap::IndexSet<PathBuf> = requires
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            let resolved = if p.is_relative() {
                proj.root().join(&p)
            } else {
                p
            };
            if is_encrypted_path(&resolved) {
                to_decrypted_path(&resolved).unwrap_or(resolved)
            } else {
                resolved
            }
        })
        .collect();

    if let Some(vars) = vars {
        requires.extend(vars.iter().filter_map(|(_, val)| {
            let p = PathBuf::from(val);
            let p = if p.is_relative() {
                proj.root().join(&p)
            } else {
                p
            };
            if to_encrypted_path(&p).exists() {
                Some(p)
            } else {
                None
            }
        }));
    }

    if requires.is_empty() {
        return Ok(req_decrypted);
    }

    let status_opts = StatusOptions {
        skip_encryption: false,
        skip_decryption: false,
    };

    for req_dec_path in requires.iter() {
        if req_dec_path.exists()
            && let Some(res) = status_file(req_dec_path, status_opts)?
        {
            anyhow::bail!(
                "{}: {} is dirty, please run `ctg sync` or `ctg encrypt` first",
                "pending sync".red(),
                proj.relative_to_cwd(&res.input).display()
            );
        }
    }

    let dec_opts = DecryptOptions {
        identities: identities.to_vec(),
        recipients: recipients.to_vec(),
        dry_run: false,
        skip_gitignore,
        skip_timestamps: false,
        skip_verify_encrypted: false,
        skip_verify_recipients: false,
    };

    for req_dec_path in requires.iter() {
        let req_enc_path = to_encrypted_path(req_dec_path);
        let temp_file = TempDecryptedFile::new(req_dec_path.clone(), false);

        if !temp_file.was_present() && decrypt_file(&req_enc_path, &dec_opts)?.is_some() {
            req_decrypted.push(temp_file);
            log::info!(
                "decrypted requirement {} into {} for upstream: {}",
                req_enc_path.display(),
                req_dec_path.display(),
                upstream_name
            );
        }
    }
    Ok(req_decrypted)
}

pub(crate) fn clean_decrypted_secrets(
    req_decrypted: Vec<TempDecryptedFile>,
    upstream_name: &str,
    kind: OperationKind,
) -> Result<()> {
    let action = match kind {
        OperationKind::Pull => "pull from",
        OperationKind::Push => "push to",
        _ => "operation with",
    };

    for mut temp_file in req_decrypted {
        if let Some(res) = temp_file.cleanup(false)? {
            log::info!(
                "cleaned decrypted requirement {} after {} upstream '{}'",
                res.input.display(),
                action,
                upstream_name
            );
        }
    }
    Ok(())
}
