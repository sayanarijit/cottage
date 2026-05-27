use crate::{
    CleanOptions, DecryptOptions, DiffOptions, EditOptions, EncryptOptions, EnvOptions, Project,
    PullOptions, PushOptions, RunOptions, StatusOptions, SyncOptions, VerifyOptions, clean_path,
    decrypt_path, diff, edit as edit_task, encrypt_path, env as env_task, load_identities,
    load_recipients, print_result, pull_path, push_path, run as run_task, status_path, sync_path,
    verify_path,
};
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap::builder::styling::*;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use colored::Colorize;
use std::io::Write;
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
    #[command(name = "verify")]
    Verify(VerifyArgs),

    /// Delete all secrets and identity files.
    #[command(name = "clean", aliases = ["cl"])]
    Clean(CleanArgs),

    /// Decrypt secrets, run a command and delete decrypted secrets.
    #[command(name = "run", trailing_var_arg = true)]
    Run(RunArgs),

    /// Run command with decrypted secrets exported as environment variables.
    #[command(name = "env", trailing_var_arg = true)]
    Env(EnvArgs),

    /// Pull secrets from upstream.
    #[command(name = "pull")]
    Pull(PullArgs),

    /// Push secrets to upstream.
    #[command(name = "push")]
    Push(PushArgs),

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

    #[arg(long)]
    /// Also delete encrypted files.
    encrypted: bool,

    #[arg(long)]
    /// Also remove entries from .gitignore.
    gitignore: bool,

    #[arg(long)]
    /// Cleanup everything cottage ever did.
    all: bool,

    /// Dry run, don't actually delete anything.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
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
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
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

    /// Force re-encryption/re-decryption even if the files are not modified.
    #[arg(long, short, env = "COTTAGE_FORCE")]
    force: bool,

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
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
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
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
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
    command: String,

    /// Additional arguments to the command. If any argument is an encrypted file, it will be
    /// decrypted and replaced with the decrypted path.
    args: Vec<String>,

    /// Verify against recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Skip checksum verification and decrypt all files.
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
struct EnvArgs {
    /// The command to run.
    command: String,

    /// Additional arguments to the command.
    args: Vec<String>,

    /// Optional path to the encrypted file.
    /// Dotenv incompatible secrets will be exported as "COTTAGE_SECRET".
    /// Defaults to .env.cott.age in the current directory.
    #[arg(short = 'F', long)]
    file: Option<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

    /// Verify against recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Skip checksum verification and decrypt all files.
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

    /// Dry run, decrypt secrets in memory, but don't actually run the command.
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
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
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
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
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

#[derive(clap::Args, Debug)]
struct PullArgs {
    /// The upstream name to pull from. Defaults to all upstreams.
    upstream: Option<String>,

    /// The file or dir to pull, defaults to project root.
    path: Vec<PathBuf>,

    /// Encrypt to recipients listed at PATH. Can be repeated.
    /// Defaults to recipients in .cottage/recipients.
    #[arg(short = 'R', long, env = "COTTAGE_RECIPIENTS_FILE")]
    recipients_file: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
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

    /// Skip preview generation.
    #[arg(long, env = "COTTAGE_SKIP_PREVIEW")]
    skip_preview: bool,

    /// Dry run, don't actually pull anything.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Enable outputs from stderr, useful for debugging upstream scripts.
    #[arg(long)]
    debug: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
}

#[derive(clap::Args, Debug)]
struct PushArgs {
    /// The upstream name to push to. Defaults to all upstreams.
    upstream: Option<String>,

    /// The file or dir to push, defaults to project root.
    path: Vec<PathBuf>,

    /// Use the identity file at PATH. Can be repeated.
    /// Defaults to .cottage/identity or ~/.config/cottage/identity or ~/.ssh.
    #[arg(short, long, env = "COTTAGE_IDENTITY")]
    identity: Vec<PathBuf>,

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

    /// Dry run, don't actually push anything.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Enable outputs from stderr, useful for debugging upstream scripts.
    #[arg(long)]
    debug: bool,

    /// Compact output.
    #[arg(long, env = "COTTAGE_COMPACT")]
    compact: bool,
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
        skip_preview: args.skip_preview,
        force: args.force,
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
            encrypted: false,
        };

        for res in input.iter().flat_map(|p| clean_path(p, &clean_opts)) {
            let res = res?;
            if !quiet {
                print_result(&mut stdout, proj, &res, args.compact)?;
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
        force: args.force,
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
        gitignore: (args.all || args.gitignore) && proj.git().is_some(),
        encrypted: args.all || args.encrypted,
    };

    let mut stdout = std::io::stdout();
    for res in input.iter().flat_map(|p| clean_path(p, &opts)) {
        let res = res?;
        if !quiet {
            print_result(&mut stdout, proj, &res, args.compact)?;
        }
    }

    if args.all {
        proj.clean(args.dry_run)?;
    }
    Ok(())
}

fn run_run_cmd(proj: &Project, args: RunArgs, quiet: bool) -> Result<()> {
    let identities = load_identities(proj, args.identity).collect();
    let recipients = load_recipients(proj, args.recipients_file, None).collect();

    let options = RunOptions {
        command: args.command,
        args: args.args,
        decrypt_options: DecryptOptions {
            identities,
            recipients,
            skip_gitignore: true,
            skip_timestamps: true,
            skip_verify_encrypted: args.force || args.skip_verify_encrypted,
            skip_verify_recipients: args.force || args.skip_verify_recipients,
            dry_run: args.dry_run,
        },
        dry_run: args.dry_run,
    };

    let res = run_task(proj.root(), |p| proj.relative_to_cwd(p), options)?;
    let mut stderr = std::io::stderr();
    for op_res in res.operation_results {
        let op_res = op_res?;
        if !quiet {
            print_result(&mut stderr, proj, &op_res, args.compact)?;
        }
    }

    if res.exit_code != 0 {
        std::process::exit(res.exit_code);
    }

    Ok(())
}

fn run_env_cmd(proj: &Project, args: EnvArgs) -> Result<()> {
    let identities = load_identities(proj, args.identity).collect();
    let recipients = load_recipients(proj, args.recipients_file, None).collect();

    let options = EnvOptions {
        command: args.command,
        args: args.args,
        file: args.file,
        decrypt_options: DecryptOptions {
            identities,
            recipients,
            skip_gitignore: true,
            skip_timestamps: true,
            skip_verify_encrypted: args.force || args.skip_verify_encrypted,
            skip_verify_recipients: args.force || args.skip_verify_recipients,
            dry_run: args.dry_run,
        },
        dry_run: args.dry_run,
    };

    env_task(proj, options)
}

fn run_edit_cmd(proj: &Project, args: EditArgs, quiet: bool) -> Result<()> {
    let recipients: Vec<_> = load_recipients(proj, args.recipients_file, None).collect();
    let identities: Vec<_> = load_identities(proj, args.identity).collect();

    let options = EditOptions {
        path: args.path,
        decrypt_options: DecryptOptions {
            identities: identities.clone(),
            recipients: recipients.clone(),
            skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
            skip_timestamps: args.skip_timestamps,
            skip_verify_encrypted: args.force || args.skip_verify_encrypted,
            skip_verify_recipients: args.force || args.skip_verify_recipients,
            dry_run: false,
        },
        encrypt_options: EncryptOptions {
            recipients,
            identities,
            identity_path: proj.identity_path().to_path_buf(),
            armor: args.armor,
            skip_gitignore: args_skip_gitignore(proj, args.skip_gitignore),
            skip_timestamps: args.skip_timestamps,
            skip_preview: args.skip_preview,
            force: args.force,
            dry_run: false,
        },
        clean: args.clean,
    };

    let res = edit_task(options)?;
    let mut stdout = std::io::stdout();
    for op_res in res {
        let op_res = op_res?;
        if !quiet {
            print_result(&mut stdout, proj, &op_res, args.compact)?;
        }
    }

    Ok(())
}

fn run_pull_cmd(proj: &Project, args: PullArgs, _quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let recipients = load_recipients(proj, args.recipients_file, None).collect();
    let identities = load_identities(proj, args.identity).collect();

    let options = PullOptions {
        upstream: args.upstream,
        recipients,
        identities,
        armor: args.armor,
        skip_preview: args.skip_preview,
        identity_path: proj.identity_path().to_path_buf(),
        dry_run: args.dry_run,
        debug: args.debug,
    };

    let mut stdout = std::io::stdout();
    for path in &input {
        for res in pull_path(path, proj, &options) {
            let res = res?;
            print_result(&mut stdout, proj, &res, args.compact)?;
        }
    }
    Ok(())
}

fn run_push_cmd(proj: &Project, args: PushArgs, _quiet: bool) -> Result<()> {
    let input = get_input_paths(proj, args.path);
    let recipients = load_recipients(proj, args.recipients_file, None).collect();
    let identities = load_identities(proj, args.identity).collect();

    let options = PushOptions {
        upstream: args.upstream,
        recipients,
        identities,
        dry_run: args.dry_run,
        debug: args.debug,
    };

    let mut stdout = std::io::stdout();
    for path in &input {
        for res in push_path(path, proj, &options) {
            let res = res?;
            print_result(&mut stdout, proj, &res, args.compact)?;
        }
    }
    Ok(())
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

fn run_verify_cmd(proj: &Project, args: VerifyArgs, quiet: bool) -> Result<()> {
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

    if !quiet {
        println!(
            "{}: all encrypted secrets are in sync with metadata",
            "verified".green()
        );
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
        Command::Verify(args) => run_verify_cmd(&proj, args, is_silent),
        Command::Clean(args) => run_clean_cmd(&proj, args, is_silent),
        Command::Edit(args) => run_edit_cmd(&proj, args, is_silent),
        Command::Run(args) => run_run_cmd(&proj, args, is_silent),
        Command::Env(args) => run_env_cmd(&proj, args),
        Command::Pull(args) => run_pull_cmd(&proj, args, is_silent),
        Command::Push(args) => run_push_cmd(&proj, args, is_silent),
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
