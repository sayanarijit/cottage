<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://s13.gifyu.com/images/b7mxd.webp">
    <source media="(prefers-color-scheme: light)" srcset="https://s13.gifyu.com/images/b7mxl.webp">
    <img alt="The cottage logo" width="600" src="https://s13.gifyu.com/images/b7mxl.webp">
  </picture>
</p>

[![Cottage Verify](https://github.com/sayanarijit/cottage/actions/workflows/cottage-verify.yml/badge.svg)](https://github.com/sayanarijit/cottage/actions/workflows/cottage-verify.yml)
[![Crates.io Version](https://img.shields.io/crates/v/cottage)](https://crates.io/crates/cottage)
[![PyPI Version](https://img.shields.io/pypi/v/cottage)](https://pypi.org/project/cottage/)
[![NPM Version](https://img.shields.io/npm/v/%40sayanarijit%2Fcottage)](https://www.npmjs.com/package/@sayanarijit/cottage)
[![Docker Image Version](https://img.shields.io/docker/v/sayanarijit/cottage?label=docker)](https://hub.docker.com/r/sayanarijit/cottage)

**cott*age*** is a GitOps tool for teams to manage [age-encrypted](https://age-encryption.org/) secrets in git repositories.

It provides a simple workflow to encrypt/decrypt secrets, manage recipients, and keep
secrets out of the repo while still allowing for easy sharing via VCS. cottage also
generates redacted previews of encrypted secrets for better visibility and supports both
persistent and temporary decryption workflows, while ensuring secrets are never committed
in plaintext.

![Intro Demo](https://vhs.charm.sh/vhs-3XYLEtFXbPinb8HXgguuZ9.gif)

1. [Features](#features)
2. [Installation](#installation)
3. [Editor Integrations](#editor-integrations)
   1. [VS Code Extension](#vs-code-extension)
4. [AI Agent Integrations](#ai-agent-integrations)
   1. [Claude Code Integration](#claude-code-integration)
   2. [GitHub Copilot Integration](#github-copilot-integration)
   3. [Codex Integration](#codex-integration)
   4. [Antigravity (agy) Integration](<#antigravity-(agy)-integration>)
   5. [Cursor Integration](#cursor-integration)
5. [Quick Start](#quick-start)
6. [GitOps](#gitops)
7. [Git Hooks](#git-hooks)
8. [Access Control](#access-control)
   1. [Rules](#rules)
   2. [Verification](#verification)
9. [Any Provider as Upstream](#any-provider-as-upstream)
   1. [Example plugins](#example-plugins)
10. [Sync with any device](#sync-with-any-device)
11. [Learn More](#learn-more)
12. [Troubleshooting](#troubleshooting)
13. [Comparison](#comparison)
    1. [age vs Other Encryption](#age-vs-other-encryption)
    2. [cottage vs SOPS](#cottage-vs-sops)
    3. [cottage vs dotenvx](#cottage-vs-dotenvx)
    4. [cottage vs agebox](#cottage-vs-agebox)

## Features

- **Exposure-safe**: Uses Rust's type system to make sure bugs can never accidentally expose secrets.
- **Team-friendly**: Share public keys (recipients) in the repo, keep private keys (identities) local.
- **Access Control**: Simple allow/deny rules to control which secrets are encrypted for which recipients.
- **Manages .gitignore**: Automatically updates `.gitignore` to keep unencrypted secrets out of the repo.
- **Previews**: Generates timestamped redacted previews of encrypted secrets for better visibility.
- **Rich diffs**: Keeps git diff clean & reviewable, while `ctg diff` shows diff of locally modified secrets with tracked encrypted counterparts.
- **Checksum verification**: Prevents tampering by verifying that encrypted secrets and recipient lists match the metadata.
- **Git hooks**: Easily set up git hooks to automatically check/encrypt secrets before commit and decrypt them after checkout.
- **Persistent secrets workflow**: `ctg decrypt/edit/sync` keeps decrypted secrets on disk.
- **Temporary secrets workflow**: `ctg run` (shortcut `ctgx`) decrypts secrets temporarily to run a command, then deletes them regardless of the command's success or failure.
- **Environment injection workflow**: `ctg env` injects decrypted secrets as environment variables to run a command, without writing them to disk at all.
- **Clean up**: `ctg clean` deletes all decrypted secrets from local repo to let you run your AI agents with a tiny bit less worry.
- **Supports jj and non-git directories**: `ctg init` turns any directory into a secret store.
- **Sync with any provider**: Lets you configure any provider with an API as the upstream, and start using `ctg pull/diff/push` like `git pull/diff/push`.
- **Sync with any device**: Secrets encrypted with cottage and managed in a git repo can be synced across devices with [Cottage Sync](https://cottage-sync.github.io).

## Installation

```bash
# rust: cargo-binstall/cargo
cargo binstall --locked cottage
cargo install --locked cottage

# python: pip/uv/uvx
pip install cottage
uv pip install cottage
uvx --from cottage ctg --version

# node: yarn/pnpm/npx
yarn global add @sayanarijit/cottage
pnpm add -g @sayanarijit/cottage
npx -p @sayanarijit/cottage ctg --version
```

Also available as docker images:

```bash
# Docker
docker run --rm -v $PWD:/app sayanarijit/cottage --version

# Podman
podman run --rm -v $PWD:/app quay.io/sayanarijit/cottage --version
```

Or download the latest release from [GitHub](https://github.com/sayanarijit/cottage/releases).

## Editor Integrations

### VS Code Extension

Use the [Cottage VS Code extension](https://github.com/sayanarijit/vscode-plugin-cottage) to install `ctg`, add Copilot safety hooks, encrypt files from the Explorer, and open `.cott.age` files through the editor workflow.

[![Cottage VS Code Extension Demo](https://s13.gifyu.com/images/bn9p8.gif)](https://marketplace.visualstudio.com/items?itemName=sayanarijit.vscode-plugin-cottage)

Install it from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=sayanarijit.vscode-plugin-cottage), or build and install it locally from [`vscode-plugin-cottage`](https://github.com/sayanarijit/vscode-plugin-cottage).

## AI Agent Integrations

All of the integrations below keep AI agents from running `ctg`/`ctgx` directly and from viewing or editing secret files: anything inside `.cottage/`, any `*.cott.*` file (encrypted `*.cott.age` blobs and redacted `*.cott.toml` previews), and any decrypted file that still has a `*.cott.age` counterpart on disk.

### Claude Code Integration

If you are using Claude Code, add [`.claude/settings.json`](.claude/settings.json) and [`.claude/hooks/deny-secrets.py`](.claude/hooks/deny-secrets.py) to your repos with secrets so Claude Code sessions handle secrets safely, or install the [claude-plugin-cottage](https://github.com/sayanarijit/claude-plugin-cottage) plugin.

### GitHub Copilot Integration

If you are using GitHub Copilot in VS Code, add [`.github/hooks/ctg-policy.json`](.github/hooks/ctg-policy.json) and [`.github/hooks/scripts/deny_ctg_command.py`](.github/hooks/scripts/deny_ctg_command.py) to your repos with secrets so Copilot sessions clean decrypted files, block direct `ctg` shell commands, and block access to secret files, or install the [vscode-plugin-cottage](https://github.com/sayanarijit/vscode-plugin-cottage) extension to set that up from VS Code.

VS Code loads [`.claude/settings.json`](.claude/settings.json) hook definitions too. If you keep both Claude and Copilot hook files in the same repo, make sure you do not accidentally run the same cleanup hook twice.

### Codex Integration

If you are using Codex, add [`.codex/hooks.json`](.codex/hooks.json) and [`.codex/hooks/deny-ctg.py`](.codex/hooks/deny-ctg.py) to your repos with secrets so Codex sessions handle secrets safely, or install the [codex-plugin-cottage](https://github.com/sayanarijit/codex-plugin-cottage) plugin.

Codex requires local hooks to be reviewed before they run. After adding the files, start Codex in the repo and use `/hooks` to review and trust the project hooks.

### Antigravity (agy) Integration

If you are using Antigravity (`agy`), add [`.agents/hooks.json`](.agents/hooks.json) and [`.agents/scripts/deny-ctg.py`](.agents/scripts/deny-ctg.py) to your repos with secrets so Antigravity sessions handle secrets safely, or install the [agy-plugin-cottage](https://github.com/sayanarijit/agy-plugin-cottage) plugin.

### Cursor Integration

If you are using Cursor, add [`.cursor/hooks.json`](.cursor/hooks.json), [`.cursor/hooks/deny-ctg.py`](.cursor/hooks/deny-ctg.py), [`.cursor/hooks/deny-read-secrets.py`](.cursor/hooks/deny-read-secrets.py), [`.cursor/rules/deny-ctg.mdc`](.cursor/rules/deny-ctg.mdc), and [`.cursorignore`](.cursorignore) to your repos with secrets so Cursor sessions handle secrets safely.

Cursor requires hooks to be enabled first. Open Cursor Settings > Hooks and enable hooks, then restart the agent session so the project hooks take effect. `.cursorignore` additionally keeps secret files out of Cursor's indexing and the Agent's context.

## Quick Start

Init project:

```bash
mkdir project && cd project

git init  # Optional, cottage works better with git but it's not required
ctg init  # Sets up the .cottage directory and necessary files

tree -a
# .
# ├ .cottage/           <- Auto-generated by `ctg init`
# │ ├ identity        <- Your private key, keep it safe. Move it to `~/.config/cottage/identity` to use it globally, or replace it with a soft link to one of your existing private keys.
# │ └ recipients/     <- This is where your team keeps the public keys of all the recipients.
# │     └ sayanarijit <- Your public key. Commit it. To use an existing public key, just copy (don't softlink) that key here.
# ├ .git/...
# ├ .gitattributes      <- Added `*.cott.age binary export-ignore filter=cottage-encrypted -diff` to avoid polluting git diff
# └ .gitignore          <- Added `/.cottage/identity` for obvious reasons

# You can run `ctg clean --all` anytime to clean up everything cottage ever did.
```

Create or edit a secret.

```bash
ctg edit secret.yml --clean    # Opens secret.yml in $EDITOR
ctg encrypt secret.yml --clean # Another way to encrypt secrets
# encrypt secret.yml
#    into secret.yml.cott.age
#    edit secret.yml.cott.toml
#    edit .gitignore
# delete secret.yml
```

Run a command with temporary decrypted secrets:

```bash
cat secret.yml
# cat: secret.yml: No such file or directory

ctg run kubectl apply -f secret.yml          # decrypts secret.yml.cott.age to secret.yml and runs the command
ctg run kubectl apply -f secret.yml.cott.age # also replaces the path argument with the decrypted file path
ctg run kubectl apply -f .                   # decrypts all .cott.age files in . and runs the command
ctg run ./deploy.sh                          # decrypts all .cott.age files in repo and runs the command

cat secret.yml
# cat: secret.yml: No such file or directory
```

Or use the shortcut:

```bash
ctgx ./deploy.sh  # same as ctg run ./deploy.sh
```

Run a command with secrets injected as environment variables, without writing to disk at all:

```bash
ctg env -- ./deploy.sh # Export secrets from .env.cott.age (default) without writing them to disk, then run deploy.sh
ctg env -F .env.prod.cott.age -- ./deploy.sh # exports from .env.prod.cott.age instead of .env.cott.age
ctg env -F secrets.json.cott.age -- printenv COTTAGE_SECRET # Also supports non-dotenv files.
```

## GitOps

To share your secrets with team members, just push to the git repo.

```bash
git add .
git commit -m "Add secret.yml"
git push origin main
```

Ask your teammates to add their public keys to `.cottage/recipients` and push the
changes. Then you can pull and re-encrypt the secrets for them.

```bash
git pull origin main

ctg decrypt --skip-verify-recipients  # Decrypt missing secrets for re-encryption
ctg encrypt                           # Re-encrypt all secrets
# encrypt secret.yml
#    into secret.yml.cott.age
#    edit secret.yml.cott.toml

ctg clean  # optional
# delete secret.yml

# review changes, commit and push
git add .
git commit -m "Add new recipient to secrets"
git push origin main
```

Now your teammates can pull the latest changes and decrypt secrets for themselves.

## Git Hooks

You can use [prek](https://github.com/j178/prek) or [pre-commit](https://pre-commit.com/) to set up git hooks to automatically check/encrypt secrets before commit and decrypt them after checkout.

See the [example prek configuration here](examples/prek.toml).

After adding the `prek.toml` file, run:

```bash
prek install
prek install --hook-type post-checkout
prek install --hook-type post-merge
prek install --hook-type post-rewrite
```

## Access Control

### Rules

In the metadata file, you can annotate which recipients the secret should be encrypted for.
This allows you to have different secrets for different environments (e.g. staging vs production) and only encrypt them for the relevant recipients.

```toml
# secret.yml.cott.toml
[secret]
allow = ["sayanarijit"]  # Only encrypt for sayanarijit
```

```toml
# secret.yml.cott.toml
[secret]
deny = ["sayanarijit"]  # Encrypt for everyone except sayanarijit
```

```toml
# secret.yml.cott.toml
[secret]
allow = ["env/staging/*"]  # Supports glob patterns, only encrypt for recipients in env/staging
deny = ["env/staging/badservice"]  # Encrypt for everyone in env/staging except badservice
```

Deny rules take precedence over allow rules.

See [metadata specification](./SPECIFICATION.md#secretmetadata) for more details.

### Verification

You can run `ctg verify` in CI to verify that the encrypted secrets and recipient lists match the metadata rules, to prevent tampering.

```yaml
# .github/workflows/cottage-verify.yml
name: Cottage Verify
on: [push, pull_request]
permissions:
  contents: read
jobs:
  verify-secrets:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Verify secrets
        run: docker run --rm -v "${{ github.workspace }}:/app" ghcr.io/sayanarijit/cottage verify
```

## Any Provider as Upstream

With cottage, you can sync secrets with any provider that has an API, not just git.

For that, create a file named `cottage.toml` in the project root and configure the upstream settings.

See the [example `cottage.toml` here](./cottage.toml) and the [secret specific upstream configuration here](./examples/secrets/secret.json.cott.toml).

See an [example plugin implementation here](./examples/plugins/cottage-plugin-vault-in-kubernetes.py).

The workflow is similar to git, but instead of `git pull` and `git push`, you run `ctg pull` and `ctg push` to sync secrets with the configured upstream.

Example:

```bash
# Pull latest changes into local encrypted secrets
# Similar to `git pull origin`
ctg pull myvault

# Compare diff with local decrypted secrets
ctg diff

# Sync local decrypted secrets with local encrypted secrets
ctg sync

# Push changes from local encrypted secrets to upstream
# Similar to `git push origin main`
ctg push myvault
```

See [upstream configuration specification](./SPECIFICATION.md#upstreamconfig) for more details.

### Example plugins

Cottage supports various plugin providers to sync your secrets. Ready-to-use plugin scripts are available in the [examples/plugins](examples/plugins/) directory:

- [1Password](examples/plugins/cottage-plugin-onepassword.py)
- [AWS Secrets Manager](examples/plugins/cottage-plugin-aws-secretsmanager.py)
- [Azure Key Vault](examples/plugins/cottage-plugin-azure-keyvault.py)
- [Bitwarden](examples/plugins/cottage-plugin-bitwarden.py)
- [Dashlane](examples/plugins/cottage-plugin-dashlane.py)
- [Doppler](examples/plugins/cottage-plugin-doppler.py)
- [ejson](examples/plugins/cottage-plugin-ejson.py)
- [Google Cloud Secret Manager](examples/plugins/cottage-plugin-gcp-secretmanager.py)
- [HashiCorp Vault](examples/plugins/cottage-plugin-vault.py) (also see [Vault in Kubernetes](examples/plugins/cottage-plugin-vault-in-kubernetes.py))
- [Keeper Security](examples/plugins/cottage-plugin-keeper.py)
- [KeePass (Passhole)](examples/plugins/cottage-plugin-passhole.py)
- [LastPass](examples/plugins/cottage-plugin-lastpass.py)
- [pass (password-store)](examples/plugins/cottage-plugin-pass.py)
- [Proton Pass](examples/plugins/cottage-plugin-protonpass.py)
- [System Keyring](examples/plugins/cottage-plugin-keyring.py)
- [Zoho Vault](examples/plugins/cottage-plugin-zoho-vault.py)

## Sync with any device

Use [Cottage Sync](https://cottage-sync.github.io) to sync your secrets across your devices and browse without needing the CLI.

## Learn More

See [examples](examples/) directory for more usage examples.

## Troubleshooting

```bash
# See debug logs with -v, -vv or -vvv
ctg run -vvv -- ./deploy.sh
```

## Comparison

### age vs Other Encryption

[age](https://age-encryption.org) uses a modern, simple algorithm optimized for secure file encryption, with a focus on usability and minimal attack surface. It also [supports SSH RSA and Ed25519 keys](https://words.filippo.io/using-ed25519-keys-for-encryption/), though it's recommended to use different keys for separate purposes and scopes.

### cottage vs SOPS

While [SOPS](https://getsops.io/) and cottage have many overlapping features, cottage has the following advantages:

- Auto manage .gitignore to ensure unencrypted secrets are never committed to git.
- Encrypted secrets being pure age encrypted .age files, allows for better interoperability with a wider ecosystem of tools.
- Cleaner diffs - unlike SOPS, which generates diffs for every value of every secret, even if the actual change is just adding/removing a recipient, cottage only generates one diff per file, explicitly pointing out the change in recipients checksum.

### cottage vs dotenvx

cottage borrows the `ctg env` API from [dotenvx](https://dotenvx.com).

- Supports any file type, not just dotenv files.
- Manages multiple secrets in a repo.
- Access control rules to encrypt secrets for specific recipients.
- Cleaner diffs - see [cottage vs SOPS](#cottage-vs-sops).

### cottage vs agebox

[agebox](https://github.com/slok/agebox) is very similar to cottage in core philosophy but lacks many [features](#features).
