use crate::{
    DecryptOptions, DecryptionMode, EncryptOptions, EncryptionMode, Project, decrypt_dir,
    decrypt_file, encrypt_dir, encrypt_file, load_identities, load_recipients, sync_dir, sync_file,
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

    /// Skip updating timestamps on encrypted files.
    #[arg(long)]
    skip_timestamps: bool,

    /// Skip adding encrypted files to .gitignore.
    #[arg(long)]
    skip_gitignore: bool,
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

    /// Skip updating timestamps on decrypted files.
    #[arg(long)]
    skip_timestamps: bool,

    /// Skip adding decrypted files to .gitignore.
    #[arg(long)]
    skip_gitignore: bool,
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

    /// Skip updating timestamps on encrypted and decrypted files.
    #[arg(long)]
    skip_timestamps: bool,

    /// Skip adding encrypted and decrypted files to .gitignore.
    #[arg(long)]
    skip_gitignore: bool,
}

fn run_encrypt_cmd(proj: &Project, args: EncryptArgs) -> Result<()> {
    let input = args.path.unwrap_or_else(|| proj.root().into());
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
        mode: mode,
        armor: args.armor,
        skip_gitignore: args.skip_gitignore,
        skip_timestamps: args.skip_timestamps,
    };
    if input.is_dir() {
        for res in encrypt_dir(&input, &options) {
            let (input, output, gi) = res?;
            println!(
                "╭─ {}",
                input.strip_prefix(proj.cwd()).unwrap_or(&input).display()
            );
            if let Some(gi) = gi {
                println!(
                    "├─ {}",
                    gi.strip_prefix(&proj.cwd()).unwrap_or(&gi).display()
                );
            }
            println!(
                "╰─ {}",
                output
                    .strip_prefix(&proj.cwd())
                    .unwrap_or(&output)
                    .display()
            );
        }
    } else if input.is_file() {
        let (input, output, gi) = encrypt_file(&input, &options)?;
        println!("╭─ {}", input.display());
        if let Some(gi) = gi {
            println!("├─ {}", gi.display());
        }
        println!("╰─ {}", output.display());
    } else if !input.exists() {
        return Err(anyhow!("Path does not exist: {}", proj.root().display()));
    }

    Ok(())
}

fn run_decrypt_cmd(proj: &Project, args: DecryptArgs) -> Result<()> {
    let input = args.path.unwrap_or_else(|| proj.root().into());
    let identities = load_identities(&proj, &args.identity)?;

    let mode = if args.passphrase {
        let pass = rpassword::prompt_password("Enter passphrase: ")?;
        DecryptionMode::Passphrase(pass)
    } else {
        DecryptionMode::Identities(&identities)
    };
    let options = DecryptOptions {
        mode: mode,
        skip_gitignore: args.skip_gitignore,
        skip_timestamps: args.skip_timestamps,
    };

    if input.is_dir() {
        for res in decrypt_dir(&input, &options) {
            let (input, output, gi) = res?;
            println!(
                "╭─ {}",
                input.strip_prefix(proj.cwd()).unwrap_or(&input).display()
            );
            if let Some(gi) = gi {
                println!(
                    "├─ {}",
                    gi.strip_prefix(proj.cwd()).unwrap_or(&gi).display()
                );
            }
            println!(
                "╰─ {}",
                output.strip_prefix(proj.cwd()).unwrap_or(&output).display()
            );
        }
    } else if input.is_file() {
        let (input, output, gi) = decrypt_file(&input, &options)?;
        println!("╭─ {}", input.display());
        if let Some(gi) = gi {
            println!(
                "├─ {}",
                gi.strip_prefix(proj.cwd()).unwrap_or(&gi).display()
            );
        }
        println!("╰─ {}", output.display());
    } else if !input.exists() {
        return Err(anyhow!("Path does not exist: {}", proj.root().display()));
    }

    Ok(())
}

fn run_sync_cmd(proj: &Project, args: SyncArgs) -> Result<()> {
    let input = args.path.unwrap_or_else(|| proj.root().into());
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

    let enc_options = EncryptOptions {
        mode: enc_mode,
        armor: args.armor,
        skip_gitignore: args.skip_gitignore,
        skip_timestamps: args.skip_timestamps,
    };
    let dec_options = DecryptOptions {
        mode: dec_mode,
        skip_gitignore: args.skip_gitignore,
        skip_timestamps: args.skip_timestamps,
    };

    if input.is_dir() {
        for res in sync_dir(&input, &enc_options, &dec_options) {
            let (input, output, gi) = res?;
            println!(
                "╭─ {}",
                input.strip_prefix(proj.cwd()).unwrap_or(&input).display()
            );
            if let Some(gi) = gi {
                println!(
                    "├─ {}",
                    gi.strip_prefix(proj.cwd()).unwrap_or(&gi).display()
                );
            }

            println!(
                "╰─ {}",
                output.strip_prefix(proj.cwd()).unwrap_or(&output).display()
            );
        }
    } else if input.is_file() {
        if let Some((input, output, gi)) = sync_file(&input, &enc_options, &dec_options)? {
            println!("╭─ {}", input.display());
            if let Some(gi) = gi {
                println!(
                    "├─ {}",
                    gi.strip_prefix(proj.cwd()).unwrap_or(&gi).display()
                );
            }
            println!("╰→ {}", output.display());
        }
    } else if !input.exists() {
        return Err(anyhow!("Path does not exist: {}", input.display()));
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
    }
}
