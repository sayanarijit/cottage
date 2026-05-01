use crate::dec::decrypt_file;
use crate::enc::encrypt_file;
use crate::{
    CleanOptions, DecryptOptions, DecryptionMode, DiffOptions, EncryptOptions, EncryptionMode,
    OperationKind, Project, SyncOptions, clean_path, clean_project, decrypt_path, diff,
    encrypt_path, is_encrypted_path, is_metadata_path, load_identities, load_recipients,
    status_path, sync_path, to_decrypted_path, to_encrypted_path,
};
use anyhow::{Result, anyhow};
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use owo_colors::OwoColorize;
use std::fs::File;
use std::io::{IsTerminal, Write, stdin};
use std::path::{Path, PathBuf};

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
struct CottageCli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The file to edit or sync with stdin.
    path: Option<PathBuf>,

    #[command(flatten)]
    verbosity: Verbosity<WarnLevel>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    // ... (rest of the file remains same, I will use a more targeted replace)
    /// Edit a file and encrypt it.
    #[command(name = "edit", aliases = ["ed"])]
    Edit(EditArgs),

    /// Encrypt files.
    #[command(name = "encrypt", aliases = ["e", "enc"])]
    Encrypt(EncryptArgs),

    /// Decrypt files.
    #[command(name = "decrypt", aliases = ["d", "dec"])]
    Decrypt(DecryptArgs),

    /// Sync encrypted and decrypted files.
    #[command(name = "sync", aliases = ["s", "syn"])]
    Sync(SyncArgs),

    /// See status of encrypted and decrypted files.
    #[command(name = "status", aliases = ["st"])]
    Status(StatusArgs),

    /// See diff between encrypted and decrypted files.
    #[command(name = "diff", aliases = ["df"])]
    Diff(DiffArgs),

    /// Delete all secrets and identity files.
    #[command(name = "clean", aliases = ["cln"])]
    Clean(CleanArgs),
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
    #[arg(long)]
    skip_gitignore: bool,

    /// Compact output.
    #[arg(long)]
    compact: bool,
}

#[derive(clap::Args, Debug, Default)]
struct EditArgs {
    /// The file to edit.
    path: PathBuf,

    /// Encrypt/decrypt with a passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Encrypt to the specified RECIPIENT. Can be repeated.
    #[arg(short, long)]
    recipient: Vec<String>,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients if not specified.
    #[arg(short = 'R', long)]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to identity in .cottage/identity if not specified.
    #[arg(short, long)]
    identity: Vec<PathBuf>,

    /// Encrypt to a PEM encoded format.
    #[arg(short, long)]
    armor: bool,

    /// Skip updating timestamps on encrypted and decrypted files.
    #[arg(long)]
    skip_timestamps: bool,

    /// Skip adding encrypted and decrypted files to .gitignore.
    #[arg(long)]
    skip_gitignore: bool,

    /// Force re-encryption even if the decrypted file is not modified.
    #[arg(long, short)]
    force: bool,

    /// Skip preview generation.
    #[arg(long)]
    skip_preview: bool,

    /// Compact output.
    #[arg(long)]
    compact: bool,
}

impl EditArgs {
    fn default_with_path(path: PathBuf) -> Self {
        EditArgs {
            path,
            ..Default::default()
        }
    }
}

#[derive(clap::Args, Debug)]
struct EncryptArgs {
    /// The file or dir to encrypt, defaults to project root.
    path: Vec<PathBuf>,

    /// Encrypt with a passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Encrypt to the specified RECIPIENT. Can be repeated.
    #[arg(short, long)]
    recipient: Vec<String>,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients if not specified.
    #[arg(short = 'R', long)]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to identity in .cottage/identity if not specified.
    #[arg(short, long)]
    identity: Vec<PathBuf>,

    /// Encrypt to a PEM encoded format.
    #[arg(short, long)]
    armor: bool,

    /// Skip updating timestamps on encrypted files.
    #[arg(long)]
    skip_timestamps: bool,

    /// Skip adding encrypted files to .gitignore.
    #[arg(long)]
    skip_gitignore: bool,

    /// Skip matching checksum and re-encrypt every files.
    #[arg(long, short)]
    force: bool,

    /// Skip preview generation.
    #[arg(long)]
    skip_preview: bool,

    /// Compact output.
    #[arg(long)]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct DecryptArgs {
    /// The file to dir to decrypt, defaults to project root.
    path: Vec<PathBuf>,

    /// Decrypt with a passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to identity in .cottage/identity if not specified.
    #[arg(short, long)]
    identity: Vec<PathBuf>,

    /// Skip updating timestamps on decrypted files.
    #[arg(long)]
    skip_timestamps: bool,

    /// Skip adding decrypted files to .gitignore.
    #[arg(long)]
    skip_gitignore: bool,

    /// Skip checksum verification and re-decrypt every files.
    #[arg(long, short)]
    force: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long)]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of decrypted files.
    #[arg(long)]
    skip_verify_decrypted: bool,

    /// Compact output.
    #[arg(long)]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct SyncArgs {
    /// The file to dir to sync, defaults to project root.
    path: Vec<PathBuf>,

    /// Encrypt with a passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Encrypt to the specified RECIPIENT. Can be repeated.
    #[arg(short, long)]
    recipient: Vec<String>,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients if not specified.
    #[arg(short = 'R', long)]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to identity in .cottage/identity if not specified.
    #[arg(short, long)]
    identity: Vec<PathBuf>,

    /// Encrypt to a PEM encoded format.
    #[arg(short, long)]
    armor: bool,

    /// Skip updating timestamps on encrypted and decrypted files.
    #[arg(long)]
    skip_timestamps: bool,

    /// Skip adding encrypted and decrypted files to .gitignore.
    #[arg(long)]
    skip_gitignore: bool,

    /// Skip preview generation.
    #[arg(long)]
    skip_preview: bool,

    /// Skip matching checksum and re-encrypt every files.
    #[arg(long)]
    force_encrypt: bool,

    /// Skip checksum verification of encrypted files.
    #[arg(long)]
    skip_verify_encypted: bool,

    /// Skip checksum verification of decrypted files.
    #[arg(long)]
    skip_verify_decrypted: bool,

    /// Skip checksum verification and re-encrypt/re-decrypt every files.
    #[arg(long, short)]
    force: bool,

    /// Compact output.
    #[arg(long)]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct DiffArgs {
    /// The file or dir to diff, defaults to project root.
    path: Vec<PathBuf>,

    /// Decrypt with a passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to identity in .cottage/identity if not specified.
    #[arg(short, long)]
    identity: Vec<PathBuf>,

    // Skip checksum verification of encrypted files.
    #[arg(long)]
    skip_verify_encrypted: bool,

    /// Skip checksum verification of decrypted files.
    #[arg(long)]
    skip_checksum_decrypted: bool,

    /// Skip checksum verification.
    #[arg(long, short)]
    force: bool,

    /// Exit with code 1 if there is any diff.
    #[arg(long)]
    fail: bool,
}

#[derive(clap::Args, Debug)]
struct StatusArgs {
    /// The file to dir to check status of, defaults to project root.
    path: Vec<PathBuf>,

    /// Compact output.
    #[arg(long)]
    compact: bool,

    /// Exit with code 1 if there are pending operations.
    #[arg(long)]
    fail: bool,
}

fn print_result(proj: &Project, kind: OperationKind, input: &Path, output: &Path, compact: bool) {
    match (kind, compact) {
        (OperationKind::Encrypt, false) => {
            println!(
                "{} {}\n   {} {}",
                "encrypt".green(),
                proj.relative_to_cwd(input).display(),
                "into".blue(),
                proj.relative_to_cwd(output).display()
            );
        }
        (OperationKind::Decrypt, false) => {
            println!(
                "{} {}\n   {} {}",
                "decrypt".cyan(),
                proj.relative_to_cwd(input).display(),
                "into".blue(),
                proj.relative_to_cwd(output).display()
            );
        }
        (OperationKind::Encrypt, true) => {
            println!("{}", proj.relative_to_cwd(output).display().green());
        }
        (OperationKind::Decrypt, true) => {
            println!("{}", proj.relative_to_cwd(output).display().cyan());
        }
    }
}

fn run_encrypt_cmd(proj: &Project, args: EncryptArgs, quiet: bool) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let recipients = load_recipients(proj, &args.recipient, &args.recipients_file)?;
    log::debug!("encrypt: loaded recipients");
    let identities = load_identities(proj, &args.identity)?;
    log::debug!("encrypt: loaded identities");

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        let confirm = rpassword::prompt_password("Confirm passphrase: ")?;
        if pass != confirm {
            return Err(anyhow!("passphrases do not match"));
        }
        EncryptionMode::Passphrase(pass)
    } else {
        EncryptionMode::Recipients(&recipients)
    };

    let options = EncryptOptions {
        mode,
        identities: &identities,
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
                print_result(proj, res.kind, &res.input, &res.output, args.compact);
            }
        }
    }

    Ok(())
}

fn run_decrypt_cmd(proj: &Project, args: DecryptArgs, quiet: bool) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let identities = load_identities(proj, &args.identity)?;
    log::debug!("decrypt: loaded identities");

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        DecryptionMode::Passphrase(pass)
    } else {
        DecryptionMode::Identities(&identities)
    };

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
                print_result(proj, res.kind, &res.input, &res.output, args.compact);
            }
        }
    }
    Ok(())
}

fn run_status_cmd(proj: &Project, args: StatusArgs, quiet: bool) -> Result<()> {
    log::debug!("status: checking paths: {:?}", args.path);
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let mut has_pending = false;
    for path in &input {
        for res in status_path(path) {
            let res = res?;
            has_pending = true;
            if !quiet {
                print_result(proj, res.kind, &res.input, &res.output, args.compact);
            }
        }
    }

    if has_pending && args.fail {
        std::process::exit(1);
    }

    Ok(())
}

fn run_sync_cmd(proj: &Project, args: SyncArgs, quiet: bool) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let recipients = load_recipients(proj, &args.recipient, &args.recipients_file)?;
    log::debug!("encrypt: loaded recipients");
    let identities = load_identities(proj, &args.identity)?;
    log::debug!("encrypt: loaded identities");

    let (enc_mode, dec_mode) = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        (
            EncryptionMode::Passphrase(pass.clone()),
            DecryptionMode::Passphrase(pass),
        )
    } else {
        (
            EncryptionMode::Recipients(&recipients),
            DecryptionMode::Identities(&identities),
        )
    };

    let sync_options = SyncOptions {
        encryption_mode: enc_mode,
        decryption_mode: dec_mode,
        identities: &identities,
        armor: args.armor,
        skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
        skip_timestamps: args.skip_timestamps,
        skip_preview: args.skip_preview,
        skip_verify_encrypted: args.force || args.skip_verify_encypted,
        skip_verify_decrypted: args.force || args.skip_verify_decrypted,
        force_encrypt: args.force || args.force_encrypt,
    };

    for path in &input {
        for res in sync_path(path, &sync_options) {
            let res = res?;
            if !quiet {
                print_result(proj, res.kind, &res.input, &res.output, args.compact);
            }
        }
    }

    Ok(())
}

fn run_diff_cmd(proj: &Project, args: DiffArgs) -> Result<()> {
    log::debug!("diff: checking paths: {:?}", args.path);
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let identities = load_identities(proj, &args.identity)?;
    log::debug!("diff: loaded identities");

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        DecryptionMode::Passphrase(pass)
    } else {
        DecryptionMode::Identities(&identities)
    };

    let options = DiffOptions {
        mode,
        skip_verify_encrypted: args.force || args.skip_verify_encrypted,
        skip_checksum_decrypted: args.force || args.skip_checksum_decrypted,
    };

    if diff(proj, &input, &options)? && args.fail {
        std::process::exit(1);
    }

    Ok(())
}

fn run_clean_cmd(proj: &Project, args: CleanArgs, quiet: bool) -> Result<()> {
    log::debug!("clean: checking paths: {:?}", args.path);

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

    let passphrase = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        Some(pass)
    } else {
        None
    };

    let identities = load_identities(proj, &args.identity)?;
    let recipients = load_recipients(proj, &args.recipient, &args.recipients_file)?;

    if !stdin().is_terminal() {
        let mut outfile = File::create(&decrypted_path)?;
        let mut writer = std::io::BufWriter::new(&mut outfile);
        let infile = stdin().lock();
        let mut reader = std::io::BufReader::new(infile);
        std::io::copy(&mut reader, &mut writer)?;
        writer.flush()?;
    } else {
        if is_target_encrypted {
            let mode = if let Some(pass) = passphrase.clone() {
                DecryptionMode::Passphrase(pass)
            } else {
                DecryptionMode::Identities(&identities)
            };
            let options = DecryptOptions {
                mode,
                skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
                skip_timestamps: args.skip_timestamps,
                skip_verify_encrypted: false,
                skip_verify_decrypted: false,
            };

            let _ = decrypt_file(&encrypted_path, &options)?;
        }

        edit::edit_file(&decrypted_path)?;
    }

    {
        let mode = if let Some(passphrase) = passphrase {
            EncryptionMode::Passphrase(passphrase)
        } else {
            EncryptionMode::Recipients(&recipients)
        };

        let options = EncryptOptions {
            mode,
            identities: &identities,
            armor: args.armor,
            skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
            skip_timestamps: args.skip_timestamps,
            force: false,
            skip_preview: args.skip_preview,
        };

        if let Some(res) = encrypt_file(&decrypted_path, &options)?
            && !quiet
        {
            print_result(proj, res.kind, &res.input, &res.output, args.compact);
        }
    }

    Ok(())
}

pub fn run() -> Result<()> {
    let cli = CottageCli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbosity.into())
        .format(|buf, record| {
            if record.level() <= log::Level::Warn {
                writeln!(
                    buf,
                    "{}: {}",
                    record.level().to_string().to_lowercase().yellow(),
                    record.args()
                )
            } else {
                writeln!(
                    buf,
                    "{} {}: {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").dimmed(),
                    record.level().to_string().to_lowercase().yellow(),
                    record.args()
                )
            }
        })
        .init();

    let proj = Project::init()?;

    match cli.command {
        Some(Commands::Encrypt(args)) => run_encrypt_cmd(&proj, args, cli.verbosity.is_silent()),
        Some(Commands::Decrypt(args)) => run_decrypt_cmd(&proj, args, cli.verbosity.is_silent()),
        Some(Commands::Sync(args)) => run_sync_cmd(&proj, args, cli.verbosity.is_silent()),
        Some(Commands::Status(args)) => run_status_cmd(&proj, args, cli.verbosity.is_silent()),
        Some(Commands::Diff(args)) => run_diff_cmd(&proj, args),
        Some(Commands::Clean(args)) => run_clean_cmd(&proj, args, cli.verbosity.is_silent()),
        Some(Commands::Edit(args)) => run_edit_cmd(&proj, args, cli.verbosity.is_silent()),
        None => {
            if let Some(path) = cli.path {
                let args = EditArgs::default_with_path(path);
                run_edit_cmd(&proj, args, cli.verbosity.is_silent())
            } else {
                Ok(())
            }
        }
    }
}

fn args_skip_gitignore(proj: &Project, skip_gitignore: bool) -> bool {
    skip_gitignore || proj.git().is_none()
}
