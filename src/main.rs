use anyhow::{Result, anyhow};
use cottage::{
    EncryptOptions, EncryptionMode, encrypt_dir, encrypt_file, parse_identities_dir,
    parse_identity_file, parse_recipient, parse_recipients_dir, parse_recipients_file,
};
use std::path::PathBuf;

use clap::Parser;
use std::path::Path;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CottageCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Encrypt secrets
    #[command(name = "encrypt", alias = "e")]
    Encrypt(EncryptArgs),

    /// Decrypt secrets
    #[command(name = "decrypt", alias = "d")]
    Decrypt(DecryptArgs),

    /// Sync encrypted and decrypted files
    #[command(name = "sync", alias = "s")]
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

fn get_root(cwd: &Path, dirname: &str) -> Option<PathBuf> {
    let start = cwd.canonicalize().ok()?;
    let mut current = start;
    while let Some(path) = current.parent() {
        if current.join(dirname).is_dir() {
            return Some(current.to_path_buf());
        }
        current = path.to_path_buf();
    }
    None
}

fn get_project_root(cwd: &Path) -> Option<PathBuf> {
    get_root(cwd, ".cottage").or_else(|| get_root(cwd, ".git"))
}

fn load_recipients(
    root: &Path,
    recipients: &[String],
    recipients_file: &[PathBuf],
) -> Result<Vec<Box<dyn age::Recipient + Send>>> {
    let mut result = Vec::new();

    if recipients.is_empty() && recipients_file.is_empty() {
        let default_recipients = root.join(".cottage/recipients");
        if default_recipients.is_dir() {
            result.extend(parse_recipients_dir(&default_recipients)?);
        } else if default_recipients.is_file() {
            result.extend(parse_recipients_file(&default_recipients)?);
        }
    } else {
        for r in recipients {
            result.push(parse_recipient(r)?);
        }
        for f in recipients_file {
            if f.is_dir() {
                result.extend(parse_recipients_dir(f)?);
            } else if f.is_file() {
                result.extend(parse_recipients_file(f)?);
            }
        }
    }
    Ok(result)
}

fn load_identities(root: &Path, identities: &[PathBuf]) -> Result<Vec<Box<dyn age::Identity>>> {
    let mut result = Vec::new();

    if identities.is_empty() {
        let default_identities = root.join(".cottage/identities");
        if default_identities.is_dir() {
            result.extend(parse_identities_dir(&default_identities)?);
        } else if default_identities.is_file() {
            result.push(parse_identity_file(&default_identities)?);
        } else {
            let sshdir = dirs::home_dir()
                .ok_or_else(|| anyhow!("Failed to get home directory"))?
                .join(".ssh");
            if sshdir.is_dir() {
                result.extend(parse_identities_dir(&sshdir)?);
            }
        }
    } else {
        for i in identities {
            result.push(parse_identity_file(i)?);
        }
    }
    Ok(result)
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

fn main() -> Result<()> {
    let cli = CottageCli::parse();

    match cli.command {
        Commands::Encrypt(args) => run_encrypt_cmd(args)?,
        // Commands::Decrypt(args) => run_decrypt_cmd(args),
        // Commands::Sync(args) => run_sync_command(args),
        _ => unimplemented!(),
    }

    Ok(())
}
