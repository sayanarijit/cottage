use crate::{
    DecryptOptions, Identity, Metadata, OperationKind, OperationResult, Project, RecipientData,
    StatusOptions, UpstreamMetadata, decrypt_into_memory, iter_encrypted, run_upstream_script,
    status::status_file, to_decrypted_path, to_encrypted_path, to_metadata_path,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

pub struct PushOptions {
    pub upstream: Option<String>,
    pub recipients: Vec<RecipientData>,
    pub identities: Vec<Identity>,
    pub debug: bool,
    pub dry_run: bool,
}

pub fn push_path<'a>(
    path: &'a Path,
    proj: &'a Project,
    opts: &'a PushOptions,
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
                    if upstream_metadata.push != Some(true) {
                        log::debug!(
                            "{}: skipping upstream '{}': push is not explicitly enabled",
                            path.display(),
                            upstream_name
                        );
                        return None;
                    }
                    let res = push_upstream(proj, &path, &upstream_name, &upstream_metadata, opts);
                    res.transpose()
                })
        });

    Box::new(iter)
}

pub fn push_upstream(
    proj: &Project,
    metadata_path: &Path,
    upstream_name: &str,
    upstream_metadata: &UpstreamMetadata,
    opts: &PushOptions,
) -> Result<Option<OperationResult>> {
    let resolved = proj
        .resolve_upstream(upstream_name, upstream_metadata)
        .with_context(|| {
            format!(
                "{}: could not resolve upstream '{}'",
                upstream_name,
                metadata_path.display(),
            )
        })?;

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

    let encrypted_path = to_encrypted_path(&decrypted_path);

    if !encrypted_path.exists() {
        anyhow::bail!(
            "{}: encrypted file not found for pushing",
            encrypted_path.display()
        );
    }

    log::info!(
        "{}: pushing to upstream '{}'",
        metadata_path.display(),
        upstream_name
    );

    let dec_opts = DecryptOptions {
        identities: opts.identities.clone(),
        recipients: opts.recipients.clone(),
        dry_run: true,
        skip_gitignore: true,
        skip_timestamps: true,
        skip_verify_encrypted: false,
        skip_verify_recipients: false,
    };

    let secret = decrypt_into_memory(
        std::fs::File::open(&encrypted_path).with_context(|| {
            format!(
                "{}: could not open encrypted file",
                encrypted_path.display()
            )
        })?,
        &dec_opts,
    )?;

    if opts.dry_run {
        log::info!(
            "{}: dry-run: skipping push to upstream '{}'",
            metadata_path.display(),
            upstream_name
        );
        return Ok(None);
    }

    run_upstream_script(
        proj,
        &opts.identities,
        metadata_path,
        upstream_name,
        &resolved,
        OperationKind::Push,
        Some(secret),
        opts.debug,
    )?;

    Ok(Some(OperationResult {
        kind: OperationKind::Push,
        input: encrypted_path,
        output: Some(PathBuf::from(upstream_name)),
        edits: vec![],
        cleanups: vec![],
    }))
}
