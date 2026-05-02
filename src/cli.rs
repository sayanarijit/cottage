use crate::dec::decrypt_file;
use crate::enc::encrypt_file;
use crate::{
    CleanOptions, DecryptOptions, DecryptionMode, DiffOptions, EncryptOptions, EncryptionMode,
    OperationKind, OperationResult, Project, SyncOptions, clean_path, clean_project, decrypt_path,
    diff, encrypt_path, is_encrypted_path, is_metadata_path, load_identities, load_recipients,
    status_path, sync_path, to_decrypted_path, to_encrypted_path,
};
use anyhow::{Result, anyhow};
use clap::CommandFactory;
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use owo_colors::OwoColorize;
use std::env::VarError;
use std::fs::File;
use std::io::{IsTerminal, Write, stdin};
use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
struct CottageCli {
    #[command(subcommand)]
    command: Command,

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

    /// See status of encrypted and decrypted files.
    #[command(name = "status", aliases = ["st"])]
    Status(StatusArgs),

    /// See diff between encrypted and decrypted files.
    #[command(name = "diff", aliases = ["di"])]
    Diff(DiffArgs),

    /// Delete all secrets and identity files.
    #[command(name = "clean", aliases = ["cl"])]
    Clean(CleanArgs),

    #[cfg(feature = "autocomplete")]
    /// Generate shell completions.
    /// Example: `eval "$(cottage autocomplete bash)"` to load completions for bash.
    #[command(name = "autocomplete")]
    AutoComplete {
        /// The shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum SkipChecksum {
    Encrypted,
    Decrypted,
}

#[derive(clap::Args, Debug)]
struct CleanArgs {
    /// The file or dir to clean, defaults to project root.
    path: Vec<PathBuf>,

    /// Dry run, don't actually delete anything.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Skip removing from .gitignore.
    #[arg(long, env = "COTTAGE_SKIP_GITIGNORE")]
    skip_gitignore: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
}

#[derive(clap::Args, Debug, Default)]
struct EditArgs {
    /// The file to edit.
    path: PathBuf,

    /// Encrypt/decrypt with a passphrase.
    /// If COTTAGE_PASSPHRASE environment variable is not set, it will prompt for passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Encrypt to the specified RECIPIENT. Can be repeated.
    #[arg(short, long, env = "COTTAGE_RECIPIENT")]
    recipient: Vec<String>,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    ///. and COTTAGE_PASSPHRASE environment variable is set, it will skip using
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

    /// Skip matching checksum and re-encrypt all files.
    #[arg(long, env = "COTTAGE_FORCE_ENCRYPT")]
    force_encrypt: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of decrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_DECRYPTED")]
    skip_verify_decrypted: bool,

    /// Skip preview generation.
    #[arg(long, env = "COTTAGE_SKIP_PREVIEW")]
    skip_preview: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct EncryptArgs {
    /// The file or dir to encrypt, defaults to project root.
    path: Vec<PathBuf>,

    /// Encrypt with a passphrase.
    /// If COTTAGE_PASSPHRASE environment variable is not set, it will prompt for passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Encrypt to the specified RECIPIENT. Can be repeated.
    #[arg(short, long, env = "COTTAGE_RECIPIENT")]
    recipient: Vec<String>,

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

    /// Skip matching checksum and re-encrypt all files.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Skip preview generation.
    #[arg(long, env = "COTTAGE_SKIP_PREVIEW")]
    skip_preview: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct DecryptArgs {
    /// The file or dir to decrypt, defaults to project root.
    path: Vec<PathBuf>,

    /// Decrypt with a passphrase.
    /// If COTTAGE_PASSPHRASE environment variable is not set, it will prompt for passphrase.
    #[arg(short, long)]
    passphrase: bool,

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

    /// Skip checksum verification of decrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_DECRYPTED")]
    skip_verify_decrypted: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct SyncArgs {
    /// The file or dir to sync, defaults to project root.
    path: Vec<PathBuf>,

    /// Encrypt with a passphrase.
    /// If COTTAGE_PASSPHRASE environment variable is not set, it will prompt for passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Encrypt to the specified RECIPIENT. Can be repeated.
    #[arg(short, long, env = "COTTAGE_RECIPIENT")]
    recipient: Vec<String>,

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

    /// Skip matching checksum and re-encrypt all files.
    #[arg(long, env = "COTTAGE_FORCE_ENCRYPT")]
    force_encrypt: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of decrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_DECRYPTED")]
    skip_verify_decrypted: bool,

    /// Skip checksum verification and re-encrypt/re-decrypt all files.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct DiffArgs {
    /// The file or dir to diff, defaults to project root.
    path: Vec<PathBuf>,

    /// Decrypt with a passphrase.
    /// If COTTAGE_PASSPHRASE environment variable is not set, it will prompt for passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Skip checksum verification of encrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_ENCRYPTED")]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of decrypted files.
    #[arg(long, env = "COTTAGE_SKIP_VERIFY_DECRYPTED")]
    skip_verify_decrypted: bool,

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

    /// Exit with code 1 if there are pending operations.
    #[arg(long, env = "COTTAGE_FAIL")]
    fail: bool,
}

fn prompt_passphrase() -> Result<String> {
    let pass = rpassword::prompt_password("Enter passphrase: ")?;
    let confirm = rpassword::prompt_password("Confirm passphrase: ")?;
    if pass != confirm {
        return Err(anyhow!("mismatch: passphrase confirmation does not match"));
    }
    Ok(pass)
}

fn choose_encryption_mode(
    proj: &Project,
    use_passphrase: bool,
    passphrase: Option<String>,
    recipients: Vec<String>,
    recipients_file: Vec<PathBuf>,
) -> Result<EncryptionMode> {
    let env_passphrase = std::env::var("COTTAGE_PASSPHRASE");
    match (use_passphrase, passphrase, env_passphrase) {
        (true, Some(pass), _) => Ok(EncryptionMode::Passphrase(pass)),
        (true, None, Ok(pass)) => Ok(EncryptionMode::Passphrase(pass)),
        (true, None, Err(VarError::NotPresent)) => {
            let pass = prompt_passphrase()?;
            Ok(EncryptionMode::Passphrase(pass))
        }
        (true, _, Err(e)) => Err(anyhow!(e.to_string())),
        (false, _, _) => {
            let recips = load_recipients(proj, recipients, recipients_file).collect();
            Ok(EncryptionMode::Recipients(recips))
        }
    }
}

fn choose_decryption_mode(
    proj: &Project,
    use_passphrase: bool,
    passphrase: Option<String>,
    identities: Vec<PathBuf>,
) -> Result<DecryptionMode> {
    let env_passphrase = std::env::var("COTTAGE_PASSPHRASE");
    match (use_passphrase, passphrase, env_passphrase) {
        (true, Some(pass), _) => Ok(DecryptionMode::Passphrase(pass)),
        (true, None, Ok(pass)) => Ok(DecryptionMode::Passphrase(pass)),
        (true, None, Err(VarError::NotPresent)) => {
            let pass = prompt_passphrase()?;
            Ok(DecryptionMode::Passphrase(pass))
        }
        (true, _, Err(e)) => Err(anyhow!(e.to_string())),
        (false, _, _) => {
            let ids = load_identities(proj, identities).collect();
            Ok(DecryptionMode::Identities(ids))
        }
    }
}

fn print_result(proj: &Project, op: &OperationResult, compact: bool) {
    match (op.kind, compact) {
        (OperationKind::Encrypt, false) => {
            println!(
                "{} {}\n   {} {}",
                "encrypt".green(),
                proj.relative_to_cwd(&op.input).display(),
                "into".blue(),
                proj.relative_to_cwd(&op.output).display()
            );
        }
        (OperationKind::Decrypt, false) => {
            println!(
                "{} {}\n   {} {}",
                "decrypt".cyan(),
                proj.relative_to_cwd(&op.input).display(),
                "into".blue(),
                proj.relative_to_cwd(&op.output).display()
            );
        }
        (OperationKind::Encrypt, true) => {
            println!("{}", proj.relative_to_cwd(&op.output).display().green());
        }
        (OperationKind::Decrypt, true) => {
            println!("{}", proj.relative_to_cwd(&op.output).display().cyan());
        }
    }
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

    let mode = choose_encryption_mode(
        proj,
        args.passphrase,
        None,
        args.recipient,
        args.recipients_file,
    )?;

    let passphrase = if let EncryptionMode::Passphrase(pass) = &mode {
        Some(pass.clone())
    } else {
        None
    };

    let opt_dec_mode =
        choose_decryption_mode(proj, args.passphrase, passphrase, args.identity).ok();

    let options = EncryptOptions {
        mode,
        decryption_mode: opt_dec_mode,
        armor: args.armor,
        skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
        skip_timestamps: args.skip_timestamps,
        force: args.force,
        skip_preview: args.skip_preview,
    };

    for path in &input {
        for res in encrypt_path(path, &options) {
            let res = res?;
            if !quiet {
                print_result(proj, &res, args.compact);
            }
        }
    }

    Ok(())
}

fn run_decrypt_cmd(proj: &Project, args: DecryptArgs, quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);

    let mode = choose_decryption_mode(proj, args.passphrase, None, args.identity)?;
    let options = DecryptOptions {
        mode,
        skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
        skip_timestamps: args.skip_timestamps,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_verify_decrypted: args.force || args.skip_verify_decrypted,
    };

    for path in &input {
        for res in decrypt_path(path, &options) {
            let res = res?;
            if !quiet {
                print_result(proj, &res, args.compact);
            }
        }
    }
    Ok(())
}

fn run_status_cmd(proj: &Project, args: StatusArgs, quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);

    let mut has_pending = false;
    for path in &input {
        for res in status_path(path) {
            let res = res?;
            has_pending = true;
            if !quiet {
                print_result(proj, &res.into(), args.compact);
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

    let encryption_mode = choose_encryption_mode(
        proj,
        args.passphrase,
        None,
        args.recipient,
        args.recipients_file,
    )?;

    let passphrase = if let EncryptionMode::Passphrase(pass) = &encryption_mode {
        Some(pass.clone())
    } else {
        None
    };

    let decryption_mode = choose_decryption_mode(proj, args.passphrase, passphrase, args.identity)?;

    let sync_options = SyncOptions {
        encryption_mode,
        decryption_mode,
        armor: args.armor,
        skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
        skip_timestamps: args.skip_timestamps,
        skip_preview: args.skip_preview,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_verify_decrypted: args.force || args.skip_verify_decrypted,
        force_encrypt: args.force || args.force_encrypt,
    };

    for path in &input {
        for res in sync_path(path, &sync_options) {
            let res = res?;
            if !quiet {
                print_result(proj, &res, args.compact);
            }
        }
    }

    Ok(())
}

fn run_diff_cmd(proj: &Project, args: DiffArgs) -> Result<()> {
    let input = get_input_paths(proj, args.path);

    let mode = choose_decryption_mode(proj, args.passphrase, None, args.identity)?;
    let options = DiffOptions {
        mode,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_verify_decrypted: args.force || args.skip_verify_decrypted,
    };

    if diff(proj, &input, options)? && args.fail {
        std::process::exit(1);
    }

    Ok(())
}

fn run_clean_cmd(proj: &Project, args: CleanArgs, quiet: bool) -> Result<()> {
    let opts = CleanOptions {
        dry_run: args.dry_run,
        skip_gitignore: args.skip_gitignore,
    };

    let result = if args.path.is_empty() {
        clean_project(proj, &opts)
    } else {
        Box::new(
            args.path
                .iter()
                .flat_map(|p| clean_path(p, &opts, p == proj.identity_path())),
        )
    };

    for res in result {
        let res = res?;
        if !quiet {
            if args.compact {
                println!("{}", proj.relative_to_cwd(&res).display().red());
            } else {
                println!(
                    "{} {}",
                    "deleted".red(),
                    proj.relative_to_cwd(&res).display()
                );
            }
        }
    }
    Ok(())
}

fn run_edit_cmd(proj: &Project, args: EditArgs, quiet: bool) -> Result<()> {
    let path = &args.path;
    if is_metadata_path(path) {
        return Err(anyhow!("{}: cannot edit metadata file", path.display()));
    }

    let is_target_encrypted = is_encrypted_path(path);

    let (decrypted_path, encrypted_path) = if is_target_encrypted {
        let dec = to_decrypted_path(path)
            .ok_or_else(|| anyhow!("{}: invalid encrypted path", path.display()))?;
        (dec, path.clone())
    } else {
        (path.clone(), to_encrypted_path(path))
    };

    let passphrase = if !stdin().is_terminal() {
        let mut outfile = File::create(&decrypted_path)?;
        let mut writer = std::io::BufWriter::new(&mut outfile);
        let infile = stdin().lock();
        let mut reader = std::io::BufReader::new(infile);
        std::io::copy(&mut reader, &mut writer)?;
        writer.flush()?;
        None
    } else {
        let mode = choose_decryption_mode(proj, args.passphrase, None, args.identity.clone())?;
        let passphrase = if let DecryptionMode::Passphrase(pass) = &mode {
            Some(pass.clone())
        } else {
            None
        };
        if is_target_encrypted {
            let options = DecryptOptions {
                mode,
                skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
                skip_timestamps: args.skip_timestamps,
                skip_verify_encrypted: args.force || args.skip_verify_encrypted,
                skip_verify_decrypted: args.force || args.skip_verify_decrypted,
            };
            let _ = decrypt_file(&encrypted_path, &options)?;
        }

        edit::edit_file(&decrypted_path)?;
        passphrase
    };

    {
        let mode = choose_encryption_mode(
            proj,
            args.passphrase,
            passphrase.clone(),
            args.recipient,
            args.recipients_file,
        )?;

        let dec_mode_for_preview =
            choose_decryption_mode(proj, args.passphrase, passphrase.clone(), args.identity).ok();

        let options = EncryptOptions {
            mode,
            decryption_mode: dec_mode_for_preview,
            armor: args.armor,
            skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
            skip_timestamps: args.skip_timestamps,
            force: args.force || args.force_encrypt,
            skip_preview: args.skip_preview,
        };

        if let Some(res) = encrypt_file(&decrypted_path, &options)?
            && !quiet
        {
            print_result(proj, &res, args.compact);
        }
    }

    Ok(())
}

fn run_complete_cmd(shell: clap_complete::Shell) -> Result<()> {
    let mut cmd = CottageCli::command();
    let mut out = std::io::stdout();
    clap_complete::generate(shell, &mut cmd, "cottage", &mut out);
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
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").dimmed(),
                    record.args()
                )
            }
        })
        .init();
}

pub fn run() -> Result<()> {
    let cli = CottageCli::parse();
    setup_logging(cli.verbosity);

    let proj = if matches!(cli.command, Command::Init) {
        Project::init()?
    } else {
        Project::load()?
    };

    match cli.command {
        Command::Init => Ok(()), // already initialized above
        Command::Encrypt(args) => run_encrypt_cmd(&proj, args, cli.verbosity.is_silent()),
        Command::Decrypt(args) => run_decrypt_cmd(&proj, args, cli.verbosity.is_silent()),
        Command::Sync(args) => run_sync_cmd(&proj, args, cli.verbosity.is_silent()),
        Command::Status(args) => run_status_cmd(&proj, args, cli.verbosity.is_silent()),
        Command::Diff(args) => run_diff_cmd(&proj, args),
        Command::Clean(args) => run_clean_cmd(&proj, args, cli.verbosity.is_silent()),
        Command::Edit(args) => run_edit_cmd(&proj, args, cli.verbosity.is_silent()),
        Command::AutoComplete { shell } => run_complete_cmd(shell),
    }
}

fn args_skip_gitignore(proj: &Project, skip_gitignore: bool) -> bool {
    skip_gitignore || proj.git().is_none()
}
