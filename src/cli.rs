use crate::dec::decrypt_file;
use crate::enc::encrypt_file;
use crate::{
    CleanOptions, DecryptOptions, DiffOptions, EncryptOptions, OperationKind, OperationResult,
    Project, StatusOptions, SyncOptions, VerifyOptions, clean_path, decrypt_path, diff,
    encrypt_path, is_encrypted_path, is_metadata_path, load_identities, load_recipients,
    status_path, sync_path, to_decrypted_path, to_encrypted_path, verify_path,
};
use anyhow::{Result, anyhow};
use clap::CommandFactory;
use clap::Parser;
use clap::builder::styling::*;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use colored::Colorize;
use std::fs::File;
use std::io::{IsTerminal, Write, stdin};
use std::path::PathBuf;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default().dimmed());

#[derive(clap::Parser, Debug)]
#[command(author, version, about, styles = STYLES, long_about = None, arg_required_else_help = true)]
struct CottageCli {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    verbosity: Verbosity<WarnLevel>,
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, styles = STYLES, long_about = None, arg_required_else_help = true)]
struct CottageXCli {
    #[command(flatten)]
    run: RunArgs,

    #[command(flatten)]
    verbosity: Verbosity<WarnLevel>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Initialize cottage in the current directory.
    #[command(name = "init")]
    Init,

    /// Edit a file and encrypt it.
    #[command(name = "edit", aliases = ["ed"])]
    Edit(EditArgs),

    /// Encrypt files.
    #[command(name = "encrypt", aliases = ["en", "enc"])]
    Encrypt(EncryptArgs),

    /// Decrypt files.
    #[command(name = "decrypt", aliases = ["de", "dec"])]
    Decrypt(DecryptArgs),

    /// Sync encrypted and decrypted files.
    #[command(name = "sync", aliases = ["sy", "syn"])]
    Sync(SyncArgs),

    /// See pending actions based on timestamps only.
    /// To get the actual diff, use `ctg diff`.
    #[command(name = "status", aliases = ["st"])]
    Status(StatusArgs),

    /// See diff between encrypted and decrypted files.
    #[command(name = "diff", aliases = ["di"])]
    Diff(DiffArgs),

    /// Verify the checksum matches for encrypted files and recipients.
    #[command(name = "verify", aliases = ["ve"])]
    Verify(VerifyArgs),

    /// Delete all secrets and identity files.
    #[command(name = "clean", aliases = ["cl"])]
    Clean(CleanArgs),

    /// Decrypt secrets, run a command and delete decrypted secrets.
    #[command(name = "run", trailing_var_arg = true)]
    Run(RunArgs),

    #[cfg(feature = "autocomplete")]
    /// Generate shell completions.
    /// Example: `eval "$(ctg autocomplete bash)"` to load completions for bash.
    #[command(name = "autocomplete")]
    AutoComplete {
        /// The shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(clap::Args, Debug)]
struct CleanArgs {
    /// The file or dir to clean, defaults to project root.
    path: Vec<PathBuf>,

    /// Remove from .gitignore.
    #[arg(long, env = "COTTAGE_CLEAN_GITIGNORE")]
    gitignore: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,

    /// Dry run, don't actually delete anything.
    #[arg(short = 'n', long)]
    dry_run: bool,
}

#[derive(clap::Args, Debug, Default)]
struct EditArgs {
    /// The file to edit.
    path: PathBuf,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Encrypt to a PEM encoded format.
    #[arg(short, long, env = "COTTAGE_ARMOR")]
    armor: bool,

    /// Skip updating timestamps on encrypted and decrypted files.
    #[arg(long, env = "COTTAGE_SKIP_TIMESTAMPS")]
    skip_timestamps: bool,

    /// Skip adding encrypted and decrypted files to .gitignore.
    #[arg(long, env = "COTTAGE_SKIP_GITIGNORE")]
    skip_gitignore: bool,

    /// Force re-encryption even if the decrypted file is not modified.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Skip checksum matching and re-encrypt all files.
    #[arg(long, env = "COTTAGE_FORCE_ENCRYPT")]
    force_encrypt: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of recipients.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_RECIPIENTS")]
    skip_verify_recipients: bool,

    /// Skip preview generation.
    #[arg(long, env = "COTTAGE_SKIP_PREVIEW")]
    skip_preview: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,

    /// Delete decrypted files after editing and encrypting.
    #[arg(long, env = "COTTAGE_CLEAN")]
    clean: bool,
}

#[derive(clap::Args, Debug)]
struct EncryptArgs {
    /// The file or dir to encrypt, defaults to project root.
    path: Vec<PathBuf>,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Encrypt to a PEM encoded format.
    #[arg(short, long, env = "COTTAGE_ARMOR")]
    armor: bool,

    /// Skip updating timestamps on encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_TIMESTAMPS")]
    skip_timestamps: bool,

    /// Skip adding encrypted files to .gitignore.
    #[arg(long, env = "COTTAGE_SKIP_GITIGNORE")]
    skip_gitignore: bool,

    /// Skip checksum matching and re-encrypt all files.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Skip checksum verification of recipients.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_RECIPIENTS")]
    skip_verify_recipients: bool,

    /// Skip preview generation.
    #[arg(long, env = "COTTAGE_SKIP_PREVIEW")]
    skip_preview: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,

    /// Dry run, don't actually encrypt anything.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Delete decrypted files after encrypting.
    #[arg(long, env = "COTTAGE_CLEAN")]
    clean: bool,
}

#[derive(clap::Args, Debug)]
struct DecryptArgs {
    /// The file or dir to decrypt, defaults to project root.
    path: Vec<PathBuf>,

    /// Verify against recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Skip updating timestamps on decrypted files.
    #[arg(long, env = "COTTAGE_SKIP_TIMESTAMPS")]
    skip_timestamps: bool,

    /// Skip adding decrypted files to .gitignore.
    #[arg(long, env = "COTTAGE_SKIP_GITIGNORE")]
    skip_gitignore: bool,

    /// Skip checksum verification and re-decrypt all files.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of recipients.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_RECIPIENTS")]
    skip_verify_recipients: bool,

    /// Dry run, don't actually decrypt anything.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct RunArgs {
    /// The command to run.
    #[arg(required = true)]
    command: Vec<String>,

    /// Verify against recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Skip checksum verification and re-decrypt all files.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of recipients.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_RECIPIENTS")]
    skip_verify_recipients: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,

    /// Dry run, don't actually decrypt or run the command.
    #[arg(short = 'n', long)]
    dry_run: bool,
}

#[derive(clap::Args, Debug)]
struct SyncArgs {
    /// The file or dir to sync, defaults to project root.
    path: Vec<PathBuf>,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Encrypt to a PEM encoded format.
    #[arg(short, long, env = "COTTAGE_ARMOR")]
    armor: bool,

    /// Skip updating timestamps on encrypted and decrypted files.
    #[arg(long, env = "COTTAGE_SKIP_TIMESTAMPS")]
    skip_timestamps: bool,

    /// Skip adding encrypted and decrypted files to .gitignore.
    #[arg(long, env = "COTTAGE_SKIP_GITIGNORE")]
    skip_gitignore: bool,

    /// Skip preview generation.
    #[arg(long, env = "COTTAGE_SKIP_PREVIEW")]
    skip_preview: bool,

    /// Skip checksum matching and re-encrypt all files.
    #[arg(long, env = "COTTAGE_FORCE_ENCRYPT")]
    force_encrypt: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of recipients.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_RECIPIENTS")]
    skip_verify_recipients: bool,

    /// Skip encryption.
    #[arg(long, env = "COTTAGE_SKIP_ENCRYPTION")]
    skip_encryption: bool,

    /// Skip decryption.
    #[arg(long, env = "COTTAGE_SKIP_DECRYPTION")]
    skip_decryption: bool,

    /// Skip checksum verification and re-encrypt/re-decrypt all files.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,

    /// Dry run, don't actually encrypt or decrypt anything.
    #[arg(short = 'n', long)]
    dry_run: bool,
}

#[derive(clap::Args, Debug)]
struct DiffArgs {
    /// The file or dir to diff, defaults to project root.
    path: Vec<PathBuf>,

    /// Verify against recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of recipients.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_RECIPIENTS")]
    skip_verify_recipients: bool,

    /// Skip pending encryption.
    #[arg(long, env = "COTTAGE_SKIP_ENCRYPTION")]
    skip_encryption: bool,

    /// Skip pending decryption.
    #[arg(long, env = "COTTAGE_SKIP_DECRYPTION")]
    skip_decryption: bool,

    /// Skip checksum verification.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Exit with code 1 if there is any diff.
    #[arg(long, env = "COTTAGE_FAIL")]
    fail: bool,
}

#[derive(clap::Args, Debug)]
struct StatusArgs {
    /// The file or dir to check status of, defaults to project root.
    path: Vec<PathBuf>,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,

    /// Skip pending encryption.
    #[arg(long, env = "COTTAGE_SKIP_ENCRYPTION")]
    skip_encryption: bool,

    /// Skip pending decryption.
    #[arg(long, env = "COTTAGE_SKIP_DECRYPTION")]
    skip_decryption: bool,

    /// Exit with code 1 if there are pending operations.
    #[arg(long, env = "COTTAGE_FAIL")]
    fail: bool,
}

#[derive(clap::Args, Debug)]
struct VerifyArgs {
    /// The file or dir to verify, defaults to project root.
    path: Vec<PathBuf>,

    /// Verify against recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of recipients.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_RECIPIENTS")]
    skip_verify_recipients: bool,
}

fn print_edits(mut file: impl Write, proj: &Project, op: &OperationResult) -> Result<()> {
    for path in op.metadata.iter().chain(op.gitignore.iter()) {
        writeln!(
            file,
            "   {} {}",
            "edit".yellow(),
            proj.relative_to_cwd(path).display()
        )?;
    }
    Ok(())
}

fn print_result(
    mut file: impl Write,
    proj: &Project,
    op: &OperationResult,
    compact: bool,
) -> Result<()> {
    match (op.kind, compact) {
        (OperationKind::Encrypt, false) => {
            writeln!(
                file,
                "{} {}\n   {} {}",
                "encrypt".green(),
                proj.relative_to_cwd(&op.input).display(),
                "into".blue(),
                proj.relative_to_cwd(&op.output).display()
            )?;
            print_edits(file, proj, op)?;
        }
        (OperationKind::Decrypt, false) => {
            writeln!(
                file,
                "{} {}\n   {} {}",
                "decrypt".cyan(),
                proj.relative_to_cwd(&op.input).display(),
                "into".blue(),
                proj.relative_to_cwd(&op.output).display()
            )?;
            print_edits(file, proj, op)?;
        }
        (OperationKind::Encrypt, true) => {
            writeln!(
                file,
                "{}",
                proj.relative_to_cwd(&op.output)
                    .display()
                    .to_string()
                    .green()
            )?;
        }
        (OperationKind::Decrypt, true) => {
            writeln!(
                file,
                "{}",
                proj.relative_to_cwd(&op.output)
                    .display()
                    .to_string()
                    .cyan()
            )?;
        }
    }
    Ok(())
}

fn get_input_paths(proj: &Project, path: Vec<PathBuf>) -> Vec<PathBuf> {
    if path.is_empty() {
        vec![proj.root().into()]
    } else {
        path
    }
}

fn run_encrypt_cmd(proj: &Project, args: EncryptArgs, quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let mut stdout = std::io::stdout();

    let recipients = load_recipients(proj, args.recipients_file, None).collect();
    let identities = load_identities(proj, args.identity).collect();

    let options = EncryptOptions {
        recipients,
        identities,
        armor: args.armor,
        skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
        skip_timestamps: args.skip_timestamps,
        skip_verify_recipients: args.force || args.skip_verify_recipients,
        skip_preview: args.skip_preview,
        identity_path: proj.identity_path().to_path_buf(),
        dry_run: args.dry_run,
    };

    for path in &input {
        for res in encrypt_path(path, &options) {
            let res = res?;
            if !quiet {
                print_result(&mut stdout, proj, &res, args.compact)?;
            }
        }
    }

    if args.clean {
        let clean_opts = CleanOptions {
            dry_run: options.dry_run,
            gitignore: false,
        };

        for res in input.iter().flat_map(|p| clean_path(p, &clean_opts)) {
            let res = res?;
            if !quiet {
                if args.compact {
                    println!("{}", proj.relative_to_cwd(&res).display().to_string().red());
                } else {
                    println!(
                        "{} {}",
                        "delete".red(),
                        proj.relative_to_cwd(&res).display()
                    );
                }
            }
        }
    }

    Ok(())
}

fn run_decrypt_cmd(proj: &Project, args: DecryptArgs, quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let recipients = load_recipients(proj, args.recipients_file, None).collect();
    let identities = load_identities(proj, args.identity).collect();

    let options = DecryptOptions {
        recipients,
        identities,
        skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
        skip_timestamps: args.skip_timestamps,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_verify_recipients: args.force || args.skip_verify_recipients,
        dry_run: args.dry_run,
    };

    let mut stdout = std::io::stdout();
    for path in &input {
        for res in decrypt_path(path, &options) {
            let res = res?;
            if !quiet {
                print_result(&mut stdout, proj, &res, args.compact)?;
            }
        }
    }
    Ok(())
}

fn run_status_cmd(proj: &Project, args: StatusArgs, quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let mut stdout = std::io::stdout();

    let opts = StatusOptions {
        skip_encryption: args.skip_encryption,
        skip_decryption: args.skip_decryption,
    };

    let mut has_pending = false;
    for path in &input {
        for res in status_path(path, opts) {
            let res = res?;
            has_pending = true;
            if !quiet {
                print_result(&mut stdout, proj, &res.into(), args.compact)?;
            }
        }
    }

    if has_pending && args.fail {
        std::process::exit(1);
    }

    Ok(())
}

fn run_sync_cmd(proj: &Project, args: SyncArgs, quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let identities = load_identities(proj, args.identity).collect();
    let recipients = load_recipients(proj, args.recipients_file, None).collect();

    let sync_options = SyncOptions {
        identities,
        recipients,
        armor: args.armor,
        skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
        skip_timestamps: args.skip_timestamps,
        skip_preview: args.skip_preview,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_verify_recipients: args.force || args.skip_verify_recipients,
        skip_encryption: args.skip_encryption,
        skip_decryption: args.skip_decryption,
        force_encrypt: args.force || args.force_encrypt,
        identity_path: proj.identity_path().to_path_buf(),
        dry_run: args.dry_run,
    };

    let mut stdout = std::io::stdout();
    for path in &input {
        for res in sync_path(path, &sync_options) {
            let res = res?;
            if !quiet {
                print_result(&mut stdout, proj, &res, args.compact)?;
            }
        }
    }

    Ok(())
}

fn run_diff_cmd(proj: &Project, args: DiffArgs) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let identities = load_identities(proj, args.identity).collect();
    let recipients = load_recipients(proj, args.recipients_file, None).collect();

    let options = DiffOptions {
        identities,
        recipients,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_verify_recipients: args.force || args.skip_verify_recipients,
        skip_encryption: args.skip_encryption,
        skip_decryption: args.skip_decryption,
    };

    if diff(proj, &input, options)? && args.fail {
        std::process::exit(1);
    }

    Ok(())
}

fn run_clean_cmd(proj: &Project, args: CleanArgs, quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);

    let opts = CleanOptions {
        dry_run: args.dry_run,
        gitignore: args.gitignore && proj.git().is_some(),
    };

    for res in input.iter().flat_map(|p| clean_path(p, &opts)) {
        let res = res?;
        if !quiet {
            if args.compact {
                println!("{}", proj.relative_to_cwd(&res).display().to_string().red());
            } else {
                println!(
                    "{} {}",
                    "delete".red(),
                    proj.relative_to_cwd(&res).display()
                );
            }
        }
    }
    Ok(())
}

fn run_run_cmd(proj: &Project, args: RunArgs, quiet: bool) -> Result<()> {
    let mut input_paths = vec![];
    let mut modified_args = vec![];
    for arg in args.command.iter().skip(1) {
        let p = PathBuf::from(arg);
        if is_encrypted_path(&p) && p.exists() {
            if let Some(dec) = to_decrypted_path(&p) {
                modified_args.push(dec.to_string_lossy().to_string());
            } else {
                modified_args.push(arg.clone());
            }
            input_paths.push(p);
        } else if to_encrypted_path(&p).exists() {
            modified_args.push(arg.clone());
            input_paths.push(to_encrypted_path(&p));
        } else if p.is_dir() {
            modified_args.push(arg.clone());
            input_paths.push(p);
        } else {
            modified_args.push(arg.clone());
        }
    }

    log::debug!(
        "original args: {:?}",
        args.command[1..]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    );
    log::debug!("modified args: {:?}", modified_args);
    log::debug!("input paths: {:?}", input_paths);

    let input = get_input_paths(proj, input_paths.clone());
    let status_opts = StatusOptions::default();
    for path in input.iter() {
        for res in status_path(path, status_opts) {
            let op = res?;
            if let OperationKind::Encrypt = op.kind {
                return Err(anyhow!(
                    "{}: {} is dirty, please run `ctg sync` or `ctg encrypt` first",
                    "pending encryption".red(),
                    proj.relative_to_cwd(&op.input).display()
                ));
            }
        }
    }

    let identities = load_identities(proj, args.identity).collect();
    let recipients = load_recipients(proj, args.recipients_file, None).collect();
    let dec_options = DecryptOptions {
        identities,
        recipients,
        skip_gitignore: true,
        skip_timestamps: true,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_verify_recipients: args.force || args.skip_verify_recipients,
        dry_run: args.dry_run,
    };

    let mut stderr = std::io::stderr();
    for path in &input {
        for res in decrypt_path(path, &dec_options) {
            let res = res?;
            if !quiet {
                print_result(&mut stderr, proj, &res, args.compact)?;
            }
        }
    }

    let res = if args.dry_run {
        log::info!("dry run: skipping running the command");
        Ok((true, Some(0)))
    } else {
        let mut cmd = std::process::Command::new(&args.command[0]);
        cmd.args(&modified_args);
        log::info!("running command: {:?}", &cmd);
        cmd.status().map(|s| (s.success(), s.code()))
    };

    let clean_opts = CleanOptions {
        dry_run: args.dry_run,
        gitignore: false,
    };

    for path in input.iter().map(|p| {
        if p.is_file() && is_encrypted_path(p) {
            to_decrypted_path(p).unwrap_or_else(|| p.clone())
        } else {
            p.clone()
        }
    }) {
        for res in clean_path(&path, &clean_opts) {
            let res = res?;
            if !quiet {
                if args.compact {
                    eprintln!("{}", proj.relative_to_cwd(&res).display().to_string().red());
                } else {
                    eprintln!(
                        "{} {}",
                        "delete".red(),
                        proj.relative_to_cwd(&res).display()
                    );
                }
            }
        }
    }

    let (is_success, status_code) = res?;
    if !is_success {
        std::process::exit(status_code.unwrap_or(1));
    }

    Ok(())
}

fn run_edit_cmd(proj: &Project, args: EditArgs, quiet: bool) -> Result<()> {
    let path = &args.path;
    if is_metadata_path(path) {
        return Err(anyhow!("{}: could not edit metadata file", path.display()));
    }

    let is_target_encrypted = is_encrypted_path(path);

    let (decrypted_path, encrypted_path) = if is_target_encrypted {
        let dec = to_decrypted_path(path)
            .ok_or_else(|| anyhow!("{}: invalid encrypted path", path.display()))?;
        (dec, path.clone())
    } else {
        (path.clone(), to_encrypted_path(path))
    };

    let status1 = if !stdin().is_terminal() {
        let mut outfile = File::create(&decrypted_path)?;
        let mut writer = std::io::BufWriter::new(&mut outfile);
        let infile = stdin().lock();
        let mut reader = std::io::BufReader::new(infile);
        std::io::copy(&mut reader, &mut writer)?;
        writer.flush()?;
        Ok(())
    } else {
        let recipients = load_recipients(proj, args.recipients_file.clone(), None).collect();
        let identities = load_identities(proj, args.identity.clone()).collect();

        if is_target_encrypted {
            let options = DecryptOptions {
                identities,
                recipients,
                skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
                skip_timestamps: args.skip_timestamps,
                skip_verify_encrypted: args.force || args.skip_verify_encrypted,
                skip_verify_recipients: args.force || args.skip_verify_recipients,
                dry_run: false,
            };
            let _ = decrypt_file(&encrypted_path, &options)?;
            // Cant't fail from now on
        }

        edit::edit_file(&decrypted_path)
    };

    let status2 = {
        let recipients = load_recipients(proj, args.recipients_file, None).collect();
        let identities = load_identities(proj, args.identity).collect();

        let options = EncryptOptions {
            recipients,
            identities,
            identity_path: proj.identity_path().to_path_buf(),
            armor: args.armor,
            skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
            skip_timestamps: args.skip_timestamps,
            skip_verify_recipients: args.force || args.force_encrypt || args.skip_verify_recipients,
            skip_preview: args.skip_preview,
            dry_run: false,
        };

        let mut stdout = std::io::stdout();
        let enc_status = encrypt_file(&decrypted_path, &options);
        match enc_status {
            Ok(Some(res)) if !quiet => print_result(&mut stdout, proj, &res, args.compact),
            Ok(Some(_)) => Ok(()),
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    };

    if args.clean {
        let clean_opts = CleanOptions::default();

        for res in clean_path(&decrypted_path, &clean_opts) {
            let res = res?;
            if !quiet {
                if args.compact {
                    eprintln!("{}", proj.relative_to_cwd(&res).display().to_string().red());
                } else {
                    eprintln!(
                        "{} {}",
                        "delete".red(),
                        proj.relative_to_cwd(&res).display()
                    );
                }
            }
        }
    }

    // Now fail
    status1?;
    status2
}

fn run_complete_cmd(shell: clap_complete::Shell) -> Result<()> {
    let mut cmd = CottageCli::command();
    let mut out = std::io::stdout();
    clap_complete::generate(shell, &mut cmd, "ctg", &mut out);
    Ok(())
}

fn setup_logging(verbosity: Verbosity<WarnLevel>) {
    env_logger::Builder::new()
        .filter_level(verbosity.into())
        .format(|buf, record| {
            let level = match record.level() {
                log::Level::Error => record.level().to_string().to_lowercase().red().to_string(),
                log::Level::Warn => record
                    .level()
                    .to_string()
                    .to_lowercase()
                    .yellow()
                    .to_string(),
                _ => record.level().to_string().to_lowercase(),
            };
            if record.level() <= log::Level::Warn {
                writeln!(buf, "{}: {}", level, record.args())
            } else {
                writeln!(
                    buf,
                    "{} {}: {}",
                    level,
                    chrono::Local::now()
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .dimmed(),
                    record.args()
                )
            }
        })
        .init();
}

fn run_verify_cmd(proj: &Project, args: VerifyArgs) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let recipients = load_recipients(proj, args.recipients_file, None).collect();

    let options = VerifyOptions {
        recipients,
        skip_verify_encrypted: args.skip_verify_encrypted,
        skip_verify_recipients: args.skip_verify_recipients,
    };

    for path in &input {
        for res in verify_path(path, &options) {
            res?;
        }
    }

    Ok(())
}

fn run_cmd(cmd: Command, verbosity: Verbosity<WarnLevel>) -> Result<()> {
    if let Command::AutoComplete { shell } = cmd {
        return run_complete_cmd(shell);
    };

    setup_logging(verbosity);

    let proj = if matches!(cmd, Command::Init) {
        Project::init()?
    } else {
        Project::load()?
    };

    let is_silent = verbosity.is_silent();

    match cmd {
        Command::Init | Command::AutoComplete { shell: _ } => Ok(()), // already handled
        Command::Encrypt(args) => run_encrypt_cmd(&proj, args, is_silent),
        Command::Decrypt(args) => run_decrypt_cmd(&proj, args, is_silent),
        Command::Sync(args) => run_sync_cmd(&proj, args, is_silent),
        Command::Status(args) => run_status_cmd(&proj, args, is_silent),
        Command::Diff(args) => run_diff_cmd(&proj, args),
        Command::Verify(args) => run_verify_cmd(&proj, args),
        Command::Clean(args) => run_clean_cmd(&proj, args, is_silent),
        Command::Edit(args) => run_edit_cmd(&proj, args, is_silent),
        Command::Run(args) => run_run_cmd(&proj, args, is_silent),
    }
}

pub fn runx() -> Result<()> {
    let cli = CottageXCli::parse();
    let cmd = Command::Run(cli.run);
    run_cmd(cmd, cli.verbosity)
}

pub fn run() -> Result<()> {
    let cli = CottageCli::parse();
    run_cmd(cli.command, cli.verbosity)
}

fn args_skip_gitignore(proj: &Project, skip_gitignore: bool) -> bool {
    skip_gitignore || proj.git().is_none()
}
