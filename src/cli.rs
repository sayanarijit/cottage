use crate::{
    DecryptOptions, DecryptionMode, EncryptOptions, EncryptionMode, decrypt_dir, decrypt_file,
    encrypt_dir, encrypt_file, get_project_root, load_identities, load_recipients,
};
use anyhow::{Result, anyhow};
use clap::Parser;
use std::path::PathBuf;

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
}

#[derive(clap::Args, Debug)]
struct EncryptArgs {
    /// The file or dir to encrypt, defaults to project root.
    /// If a directory is specified, all files with associated .cott.age files will be
    /// encrypted, overwriting existing .cott.age files.
    path: Option<PathBuf>,

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
}

#[derive(clap::Args, Debug)]
struct DecryptArgs {
    /// The file to dir to decrypt, defaults to project root.
    path: Option<PathBuf>,

    /// Decrypt with a passphrase.
    #[arg(short, long)]
    passphrase: bool,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to identity in .cottage/identity if not specified.
    #[arg(short, long)]
    identity: Vec<PathBuf>,
}

#[derive(clap::Args, Debug)]
struct SyncArgs {
    /// The file to dir to sync, defaults to project root.
    path: Option<PathBuf>,

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
}

fn run_encrypt_cmd(args: EncryptArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = get_project_root(&cwd).unwrap_or_else(|| cwd.clone());
    let input = args.path.as_ref().unwrap_or(&root);

    let recipients = load_recipients(&root, &args.recipient, &args.recipients_file)?;
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
        mode: mode,
        armor: args.armor,
    };
    if input.is_dir() {
        for res in encrypt_dir(input, &options) {
            let (input, output) = res?;
            println!(
                "╭─ {}",
                input.strip_prefix(&cwd).unwrap_or(&input).display()
            );
            println!(
                "╰→ {}",
                output.strip_prefix(&cwd).unwrap_or(&output).display()
            );
        }
    } else if input.is_file() {
        let (input, output) = encrypt_file(input, &options)?;
        println!("╭─ {}", input.display());
        println!("╰→ {}", output.display());
    } else if !input.exists() {
        return Err(anyhow!("Path does not exist: {}", root.display()));
    }

    Ok(())
}

fn run_decrypt_cmd(args: DecryptArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = get_project_root(&cwd).unwrap_or_else(|| cwd.clone());
    let input = args.path.as_ref().unwrap_or(&root);

    let identities = load_identities(&root, &args.identity)?;

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        DecryptionMode::Passphrase(pass)
    } else {
        DecryptionMode::Identities(&identities)
    };
    let options = DecryptOptions { mode: mode };

    if input.is_dir() {
        for res in decrypt_dir(input, &options) {
            let (input, output) = res?;
            println!(
                "╭─ {}",
                input.strip_prefix(&cwd).unwrap_or(&input).display()
            );
            println!(
                "╰→ {}",
                output.strip_prefix(&cwd).unwrap_or(&output).display()
            );
        }
    } else if input.is_file() {
        let (input, output) = decrypt_file(input, &options)?;
        println!("╭─ {}", input.display());
        println!("╰→ {}", output.display());
    } else if !input.exists() {
        return Err(anyhow!("Path does not exist: {}", root.display()));
    }

    Ok(())
}

pub fn run() -> Result<()> {
    let cli = CottageCli::parse();

    match cli.command {
        Commands::Encrypt(args) => run_encrypt_cmd(args),
        Commands::Decrypt(args) => run_decrypt_cmd(args),
        Commands::Sync(_args) => unimplemented!("Sync command is not implemented yet"),
    }
}
