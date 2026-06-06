use crate::{
    EncryptOptions, Identity, Metadata, OperationKind, OperationResult, Project, RecipientData,
    StatusOptions, UpstreamMetadata, clean_decrypted_secrets, decrypt_required_secrets,
    encrypt_file, iter_encrypted, run_upstream_script, status::status_file, to_decrypted_path,
    to_metadata_path,
};
use age::secrecy::SecretSlice;
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

pub struct PullOptions {
    pub upstream: Option<String>,
    pub recipients: Vec<RecipientData>,
    pub identities: Vec<Identity>,
    pub identity_path: PathBuf,
    pub armor: bool,
    pub skip_preview: bool,
    pub dry_run: bool,
    pub debug: bool,
}

pub fn pull_path<'a>(
    path: &'a Path,
    proj: &'a Project,
    opts: &'a PullOptions,
) -> Box<dyn Iterator<Item = Result<OperationResult>> + 'a> {
    let iter = iter_encrypted(path)
        .filter_map(|e| to_decrypted_path(e.path()).as_deref().map(to_metadata_path))
        .filter_map(|p| Metadata::read_from_path(&p).ok().map(|m| (p, m)))
        .filter_map(|(p, m)| m.upstream.map(|um| (p, um)))
        .flat_map(move |(path, um)| {
            um.into_iter()
                .filter_map(move |(upstream_name, upstream_metadata)| {
                    if let Some(requested) = &opts.upstream
                        && requested != &upstream_name
                    {
                        log::trace!(
                            "{}: skipping upstream '{}': not requested",
                            path.display(),
                            upstream_name
                        );
                        return None;
                    }
                    if upstream_metadata.pull != Some(true) {
                        log::debug!(
                            "{}: skipping upstream '{}': pull is not explicitly enabled",
                            path.display(),
                            upstream_name
                        );
                        return None;
                    }
                    let res = pull_upstream(proj, &path, &upstream_name, &upstream_metadata, opts);
                    res.transpose()
                })
        });

    Box::new(iter)
}

pub fn pull_upstream(
    proj: &Project,
    metadata_path: &Path,
    upstream_name: &str,
    upstream_metadata: &UpstreamMetadata,
    opts: &PullOptions,
) -> Result<Option<OperationResult>> {
    let decrypted_path = to_decrypted_path(metadata_path).with_context(|| {
        format!(
            "{}: could not determine decrypted path",
            metadata_path.display()
        )
    })?;

    let status_opts = StatusOptions {
        skip_encryption: false,
        skip_decryption: true,
    };

    if decrypted_path.exists()
        && let Some(res) = status_file(&decrypted_path, status_opts)?
    {
        anyhow::bail!(
            "{}: {} is dirty, please run `ctg sync` or `ctg encrypt` first",
            "pending encryption".red(),
            proj.relative_to_cwd(&res.input).display()
        );
    }

    let resolved = proj
        .resolve_upstream(upstream_name, upstream_metadata)
        .with_context(|| {
            format!(
                "{}: could not resolve upstream '{}'",
                upstream_name,
                metadata_path.display(),
            )
        })?;

    let pull_cfg = resolved.pull.as_ref().context(format!(
        "{upstream_name}: pull operation is not configured for this upstream"
    ))?;

    // Decrypt required secrets
    let req_decrypted = decrypt_required_secrets(
        proj,
        pull_cfg.requires.as_ref(),
        pull_cfg.vars.as_ref(),
        &opts.identities,
        &opts.recipients,
        upstream_name,
        proj.git().is_none(),
    )?;

    log::info!(
        "{}: pulling from upstream '{}'",
        metadata_path.display(),
        upstream_name
    );

    let secret = if opts.dry_run {
        log::info!(
            "{}: dry-run: skipping push to upstream '{}'",
            metadata_path.display(),
            upstream_name
        );
        SecretSlice::default()
    } else {
        run_upstream_script(
            proj,
            &opts.identities,
            metadata_path,
            upstream_name,
            &resolved,
            OperationKind::Pull,
            None,
            opts.debug,
        )?
    };

    clean_decrypted_secrets(req_decrypted, upstream_name, OperationKind::Pull)?;

    let enc_opts = EncryptOptions {
        recipients: opts.recipients.clone(),
        identities: opts.identities.clone(),
        identity_path: opts.identity_path.clone(),
        armor: opts.armor,
        skip_preview: opts.skip_preview,
        dry_run: opts.dry_run,
        skip_gitignore: true,
        skip_timestamps: true,
        force: false,
    };

    if let Some(res) = encrypt_file(&decrypted_path, &enc_opts, Some(secret))? {
        Ok(Some(OperationResult {
            kind: OperationKind::Pull,
            input: PathBuf::from(upstream_name),
            output: res.output,
            edits: res.edits,
            cleanups: res.cleanups,
        }))
    } else {
        Ok(None)
    }
}
