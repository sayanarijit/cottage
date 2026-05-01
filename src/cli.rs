use crate::dec::decrypt_file;
use crate::enc::encrypt_file;
use crate::{
    DecryptOptions, DecryptionMode, DiffOptions, EncryptOptions, EncryptionMode, OperationKind,
    Project, SyncOptions, clean_project, decrypt_path, diff, encrypt_path, is_encrypted_path,
    load_identities, load_recipients, status_path, sync_path, to_decrypted_path, to_encrypted_path,
};
use anyhow::{Result, anyhow};
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CottageCli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The file to edit
    path: Option<PathBuf>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Edit a secret
    #[command(name = "edit", aliases = ["ed"])]
    Edit(EditArgs),

    /// Encrypt secrets
    #[command(name = "encrypt", aliases = ["e", "enc"])]
    Encrypt(EncryptArgs),

    /// Decrypt secrets
    #[command(name = "decrypt", aliases = ["d", "dec"])]
    Decrypt(DecryptArgs),

    /// Sync encrypted and decrypted files
    #[command(name = "sync", aliases = ["s", "syn"])]
    Sync(SyncArgs),

    /// See status of encrypted and decrypted files
    #[command(name = "status", aliases = ["st"])]
    Status(StatusArgs),

    /// See diff between encrypted and decrypted files
    #[command(name = "diff", aliases = ["df"])]
    Diff(DiffArgs),

    /// Delete all secrets and identity files
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
    /// Dry run, don't actually delete anything.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long)]
    quiet: bool,
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

    /// Skip preview generation.
    #[arg(long)]
    skip_preview: bool,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long)]
    quiet: bool,
}

impl EditArgs {
    fn default_with_path(path: PathBuf) -> Self {
        Self {
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
    #[arg(long)]
    skip_checksum: bool,

    /// Skip preview generation.
    #[arg(long)]
    skip_preview: bool,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long)]
    quiet: bool,
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

    /// Skip checksum verification.
    #[arg(long, num_args(0..=1), value_name = "TARGET")]
    skip_checksum: Option<Option<SkipChecksum>>,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long)]
    quiet: bool,
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

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long)]
    quiet: bool,
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

    /// Skip checksum verification.
    #[arg(long, num_args(0..=1), value_name = "TARGET")]
    skip_checksum: Option<Option<SkipChecksum>>,

    /// Exit with code 1 if there is any diff.
    #[arg(short, long)]
    fail: bool,
}

#[derive(clap::Args, Debug)]
struct StatusArgs {
    /// The file to dir to check status of, defaults to project root.
    path: Vec<PathBuf>,

    /// Exit with code 1 if there are pending operations.
    #[arg(short, long)]
    fail: bool,
}

fn get_skip_checksum(arg: &Option<Option<SkipChecksum>>) -> (bool, bool) {
    match arg {
        None => (false, false),
        Some(None) => (true, true),
        Some(Some(SkipChecksum::Encrypted)) => (true, false),
        Some(Some(SkipChecksum::Decrypted)) => (false, true),
    }
}

fn print_result(proj: &Project, kind: OperationKind, input: &Path, output: &Path, verbose: bool) {
    if verbose {
        match kind {
            OperationKind::Encrypt => {
                println!(
                    "encrypt {}\n   into {}",
                    proj.relative_to_cwd(&input).display(),
                    proj.relative_to_cwd(&output).display()
                );
            }
            OperationKind::Decrypt => {
                println!(
                    "decrypt {}\n   into {}",
                    proj.relative_to_cwd(&input).display(),
                    proj.relative_to_cwd(&output).display()
                );
            }
        }
    } else {
        println!("{}", proj.relative_to_cwd(output).display());
    }
}

fn run_encrypt_cmd(proj: &Project, args: EncryptArgs) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let recipients = load_recipients(&proj, &args.recipient, &args.recipients_file)?;
    let identities = load_identities(&proj, &args.identity)?;

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        let confirm = rpassword::prompt_password("Confirm passphrase: ")?;
        if pass != confirm {
            return Err(anyhow!("Passphrases do not match"));
        }
        EncryptionMode::Passphrase(pass)
    } else {
        EncryptionMode::Recipients(&recipients)
    };

    let options = EncryptOptions {
        mode,
        identities: &identities,
        armor: args.armor,
        skip_gitignore: args.skip_gitignore || proj.git().is_none(),
        skip_timestamps: args.skip_timestamps,
        skip_checksum: args.skip_checksum,
        skip_preview: args.skip_preview,
    };

    for path in &input {
        for res in encrypt_path(path, &options) {
            let res = res?;
            if !args.quiet {
                print_result(&proj, res.kind, &res.input, &res.output, args.verbose);
            }
        }
    }

    Ok(())
}

fn run_decrypt_cmd(proj: &Project, args: DecryptArgs) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let identities = load_identities(&proj, &args.identity)?;

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        DecryptionMode::Passphrase(pass)
    } else {
        DecryptionMode::Identities(&identities)
    };

    let (skip_checksum_encrypted, skip_checksum_decrypted) = get_skip_checksum(&args.skip_checksum);

    let options = DecryptOptions {
        mode,
        skip_gitignore: args.skip_gitignore || proj.git().is_none(),
        skip_timestamps: args.skip_timestamps,
        skip_checksum_encrypted,
        skip_checksum_decrypted,
    };

    for path in &input {
        for res in decrypt_path(path, &options) {
            let res = res?;
            if !args.quiet {
                print_result(&proj, res.kind, &res.input, &res.output, args.verbose);
            }
        }
    }
    Ok(())
}

fn run_status_cmd(proj: &Project, args: StatusArgs) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let mut has_pending = false;
    for path in &input {
        for res in status_path(&path) {
            let res = res?;
            print_result(proj, res.kind, &res.input, &res.output, true);
            has_pending = true;
        }
    }

    if has_pending && args.fail {
        std::process::exit(1);
    }

    Ok(())
}

fn run_sync_cmd(proj: &Project, args: SyncArgs) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let recipients = load_recipients(&proj, &args.recipient, &args.recipients_file)?;
    let identities = load_identities(&proj, &args.identity)?;

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
        skip_gitignore: args.skip_gitignore || proj.git().is_none(),
        skip_timestamps: args.skip_timestamps,
        skip_preview: args.skip_preview,
    };

    for path in &input {
        for res in sync_path(path, &sync_options) {
            let res = res?;
            if !args.quiet {
                print_result(&proj, res.kind, &res.input, &res.output, args.verbose);
            }
        }
    }

    Ok(())
}

fn run_diff_cmd(proj: &Project, args: DiffArgs) -> Result<()> {
    let input = if args.path.is_empty() {
        vec![proj.root().into()]
    } else {
        args.path
    };

    let identities = load_identities(&proj, &args.identity)?;

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        DecryptionMode::Passphrase(pass)
    } else {
        DecryptionMode::Identities(&identities)
    };

    let (skip_checksum_encrypted, skip_checksum_decrypted) = get_skip_checksum(&args.skip_checksum);

    let options = DiffOptions {
        mode,
        skip_checksum_encrypted,
        skip_checksum_decrypted,
    };

    if diff(proj, &input, &options)? && args.fail {
        std::process::exit(1);
    }

    Ok(())
}

fn run_clean_cmd(proj: &Project, args: CleanArgs) -> Result<()> {
    for deleted in clean_project(proj, args.dry_run) {
        let deleted = deleted?;
        if !args.quiet {
            if args.verbose {
                println!("deleted {}", proj.relative_to_cwd(&deleted).display());
            } else {
                println!("{}", proj.relative_to_cwd(&deleted).display());
            }
        }
    }
    Ok(())
}

fn run_edit_cmd(proj: &Project, args: EditArgs) -> Result<()> {
    let path = &args.path;
    if !path.is_file() {
        return Err(anyhow!("Not a file: {:?}", path));
    }

    let is_target_encrypted = is_encrypted_path(path);

    let (decrypted_path, encrypted_path) = if is_target_encrypted {
        let dec = to_decrypted_path(path).ok_or_else(|| anyhow!("Invalid encrypted path"))?;
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

    if is_target_encrypted {
        let mode = if let Some(pass) = passphrase.clone() {
            DecryptionMode::Passphrase(pass)
        } else {
            DecryptionMode::Identities(&identities)
        };
        let options = DecryptOptions {
            mode,
            skip_gitignore: args.skip_gitignore || proj.git().is_none(),
            skip_timestamps: args.skip_timestamps,
            skip_checksum_encrypted: false,
            skip_checksum_decrypted: false,
        };

        _ = decrypt_file(&encrypted_path, &options)?;
    }

    edit::edit_file(&decrypted_path)?;

    {
        let recipients = load_recipients(proj, &args.recipient, &args.recipients_file)?;
        let mode = if let Some(passphrase) = passphrase {
            EncryptionMode::Passphrase(passphrase)
        } else {
            EncryptionMode::Recipients(&recipients)
        };

        let options = EncryptOptions {
            mode,
            identities: &identities,
            armor: args.armor,
            skip_gitignore: args.skip_gitignore || proj.git().is_none(),
            skip_timestamps: args.skip_timestamps,
            skip_checksum: false,
            skip_preview: args.skip_preview,
        };

        let _ = encrypt_file(&decrypted_path, &options)?;
    }

    Ok(())
}

pub fn run() -> Result<()> {
    let cli = CottageCli::parse();
    let proj = Project::init()?;

    match cli.command {
        Some(Commands::Encrypt(args)) => run_encrypt_cmd(&proj, args),
        Some(Commands::Decrypt(args)) => run_decrypt_cmd(&proj, args),
        Some(Commands::Sync(args)) => run_sync_cmd(&proj, args),
        Some(Commands::Status(args)) => run_status_cmd(&proj, args),
        Some(Commands::Diff(args)) => run_diff_cmd(&proj, args),
        Some(Commands::Clean(args)) => run_clean_cmd(&proj, args),
        Some(Commands::Edit(args)) => run_edit_cmd(&proj, args),
        None => {
            if let Some(path) = cli.path {
                run_edit_cmd(&proj, EditArgs::default_with_path(path))
            } else {
                Err(anyhow!("No command or path provided"))
            }
        }
    }
}
