use crate::{
    DecryptOptions, DecryptionMode, EncryptOptions, EncryptionMode, OperationKind, Project,
    SyncOptions, decrypt_path, encrypt_path, load_identities, load_recipients, status_path,
    sync_path,
};
use anyhow::{Result, anyhow};
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CottageCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
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
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum SkipChecksum {
    Encrypted,
    Decrypted,
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
        armor: args.armor,
        skip_gitignore: args.skip_gitignore,
        skip_timestamps: args.skip_timestamps,
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
        skip_gitignore: args.skip_gitignore,
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

    for path in &input {
        for res in status_path(&path) {
            let res = res?;
            print_result(proj, res.kind, &res.input, &res.output, true);
        }
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

    let (skip_checksum_encrypted, skip_checksum_decrypted) = get_skip_checksum(&args.skip_checksum);

    let sync_options = SyncOptions {
        encryption_mode: enc_mode,
        decryption_mode: dec_mode,
        armor: args.armor,
        skip_gitignore: args.skip_gitignore,
        skip_timestamps: args.skip_timestamps,
        skip_preview: args.skip_preview,
        skip_checksum_encrypted,
        skip_checksum_decrypted,
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

pub fn run() -> Result<()> {
    let cli = CottageCli::parse();
    let proj = Project::init()?;

    match cli.command {
        Commands::Encrypt(args) => run_encrypt_cmd(&proj, args),
        Commands::Decrypt(args) => run_decrypt_cmd(&proj, args),
        Commands::Sync(args) => run_sync_cmd(&proj, args),
        Commands::Status(args) => run_status_cmd(&proj, args),
    }
}
