---
title: "cottage (ctg)"
sub_title: "A modern git-based age-encrypted secrets manager for teams"
author: "Arijit Basu <cottage@arijitbasu.in>"
theme:
  name: dark
options:
  end_slide_shorthand: true
  h1_slide_titles: true
---

# cottage

```bash +image
cat ../logo/cottage-dark.png
```

**cott*age*** is a modern git-based age-encrypted secrets manager for teams.

This guide covers all subcommands and their options with real examples.

This document is best viewed in a terminal with [presenterm](https://github.com/mfontanini/presenterm).

> [!NOTE]
> The term _"tracked secret"_ or just _"secret"_ used in this guide refers to secrets that have been encrypted, i.e., that have a corresponding `.cott.age` file.

---

# Path Target Behavior

Most `ctg` commands can take a file or a directory as an argument.

- **File**: Operates on that specific file.
- **Directory**: Recursively operates on all tracked secrets within that directory.
- **`.cott.age` file**: Usually treated as the source for decryption or the target for status/diff.
- **`.cott.toml` file**: Metadata file. Most commands skip these directly as they are managed alongside the `.cott.age` files.

---

# `ctg init`

Initialize cottage in the current directory. This creates a `.cottage` directory for recipients and identities.

```bash +exec
git checkout . -q && ctg clean -qq

ctg init
# (Initializes the .cottage directory)
```

---

# `ctg encrypt`

Encrypt files or directories. By default, it processes all the tracked secrets in the entire project root.

Pass `--clean` to automatically delete the decrypted files after encryption.

```bash +exec
git checkout . -q && ctg clean -qq

# First, un-track a secret by deleting the .cott.age file
rm ./secrets/secret.yaml.cott.*

echo "added: line" >> ./secrets/secret.yaml

# Now encrypt the file and start tracking it again
ctg encrypt ./secrets/secret.yaml
# Output:
# encrypt ./secrets/secret.yaml
#    into ./secrets/secret.yaml.cott.age
#    edit ./secrets/secret.yaml.cott.toml
```

### Target Behavior

- **Decrypted File**: Encrypts it into a `.cott.age` file and updates metadata.
- **Directory**: Recursively finds and encrypts all decrypted secrets.
- **`.cott.age`**: Skipped (already encrypted).
- **`.cott.toml`**: Skipped (metadata).

---

# `ctg decrypt`

Decrypt files or directories.

```bash +exec
git checkout . -q && ctg clean -qq

# Decrypt a specific secret
ctg decrypt ./secrets/secret.yaml.cott.age
# Output:
# decrypt ./secrets/secret.yaml.cott.age
#    into ./secrets/secret.yaml
```

### Target Behavior

- **`.cott.age` File**: Decrypts it into the corresponding plain-text file.
- **Directory**: Recursively finds and decrypts all `.cott.age` files.
- **Decrypted File**: Skipped (already decrypted).
- **`.cott.toml`**: Skipped (metadata).

---

# `ctg status`

See pending actions based on timestamps.

```bash +exec
git checkout . -q && ctg clean -qq

# Check status
ctg status ./secrets/secret.yaml.cott.age
# Output:
# decrypt ./secrets/secret.yaml.cott.age
#    into ./secrets/secret.yaml

# Decrypt and modify to see pending encryption
ctg decrypt ./secrets/secret.yaml.cott.age -qq
echo "status: change" >> ./secrets/secret.yaml

ctg status ./secrets/secret.yaml
# Output:
# encrypt ./secrets/secret.yaml
#    into ./secrets/secret.yaml.cott.age
```

### Target Behavior

- **Any Target**: Works on any path that is part of a secret (plain-text or `.cott.age`) or a directory containing them.

---

# `ctg diff`

See the actual diff between encrypted and decrypted files. This decrypts the encrypted version in memory (safe from accidental exposure) to compare.

```bash +exec
git checkout . -q && ctg clean -qq

# Decrypt and modify a file
ctg decrypt ./secrets/secret.yaml.cott.age -qq
echo "diff: change" >> ./secrets/secret.yaml

# View the diff
ctg diff ./secrets/secret.yaml
```

Output:

```diff
diff --git a/./secrets/secret.yaml b/./secrets/secret.yaml
--- a/./secrets/secret.yaml
+++ b/./secrets/secret.yaml
@@ -1 +1,2 @@
 SECRET: foobar
+diff: change
```

### Target Behavior

- **Any Target**: Similar to `status`, it can be pointed at any file in a secret pair or a directory. It will decrypt the `.cott.age` file in memory and compare it with the on-disk plain-text file.

---

# `ctg verify`

Verify the checksum matches for encrypted files and recipients. This is useful in CI to ensure that all changes to secrets and recipients are documented properly in the metadata file.

```bash +exec
git checkout . -q && ctg clean -qq

# Verify a specific secret
ctg verify ./secrets/secret.yaml.cott.age
# verified: all encrypted secrets are in sync with metadata

# Try to verify with wrong recipients to see it fail
ctg verify ./secrets/secret.yaml.cott.age -R Cargo.toml
# Output:
# Error: ./secrets/secret.yaml.cott.toml: recipients mismatch: use --skip-verify-recipients to skip this check
```

### Target Behavior

- **`.cott.age` File**: Verifies the content checksum and recipient checksum against the metadata.
- **Directory**: Recursively finds and verifies all `.cott.age` files.
- **Decrypted File**: Verifies the corresponding `.cott.age` file.

---

# `ctg sync`

Keeps encrypted and decrypted files in sync based on timestamps.

```bash +exec
git checkout . -q && ctg clean -qq

# Decrypt and modify
ctg decrypt ./secrets/secret.yaml.cott.age -qq
echo "sync: change" >> ./secrets/secret.yaml

# Sync will encrypt the newer decrypted file
ctg sync ./secrets/secret.yaml
# Output:
# encrypt ./secrets/secret.yaml
#    into ./secrets/secret.yaml.cott.age
#    edit ./secrets/secret.yaml.cott.toml
```

### Target Behavior

- **File/Dir/Age**: Syncs the target(s). If a decrypted file is newer, it encrypts. If a `.cott.age` file is newer, it decrypts.

---

# `ctg edit`

Edit and encrypt a file directly. Opens it in your default `$EDITOR`, and re-encrypts it upon saving and exiting.

`ctg edit` decrypts the secret before opening the editor. After editing and saving, it re-encrypts the secret. If the decrypted file was already present on disk before running `ctg edit`, it is retained. If it was not present beforehand, it is cleaned up automatically.

Run it with `--clean` to ensure the decrypted file is cleaned up even if it was present before.

```bash +exec
git checkout . -q && ctg clean -qq

# ctg edit ./secrets/secret.yaml.cott.age --clean # (Opens $EDITOR, then encrypts on save)

echo foo | ctg edit ./secrets/secret.yaml.cott.age --clean

# Output:
# encrypt ./secrets/secret.yaml
#    into ./secrets/secret.yaml.cott.age
#    edit ./secrets/secret.yaml.cott.toml
# delete ./secrets/secret.yaml
```

### Target Behavior

- **Plain Text**: Directly opens the decrypted file for editing, then encrypts it after saving. Retains the plain text file on disk unless `--clean` is specified.
- **`.cott.age`**: Decrypts for editing, re-encrypts after saving, and automatically cleans up the decrypted file unless it was already present on disk beforehand.
- **Directory**: Not supported (must target a specific secret).
- **`.cott.toml`**: Not supported (cannot edit metadata directly).

---

# `ctg clean`

Delete all decrypted secrets to keep the workspace clean.

Run it with `--gitignore` to also remove the gitignore entry.

```bash +exec
git checkout . -q && ctg de ./secrets/secret.yaml.cott.age -qq

# First, dry run to see what would be deleted
ctg clean . --dry-run
# Output:
# delete ./secrets/secret.yaml

# Actually delete decrypted secrets
ctg clean .
# Output:
# delete ./secrets/secret.yaml
```

### Target Behavior

- **Decrypted File**: Deletes it.
- **Directory**: Recursively deletes all decrypted secrets within.
- **`.cott.age` / `.cott.toml`**: Skipped.

> [!WARNING]
> ctg clean is destructive for your local decrypted copies.
> Always ensure your changes are encrypted before cleaning.

---

# `ctg run` / `ctgx`

Decrypt secrets, run a specified command, and manage the decrypted secret lifecycle.

`ctg run` (shortcut `ctgx`) decrypts secrets before running the command. After the command finishes (whether it succeeds or fails), secrets that were not present before running are automatically cleaned up, while secrets that were already present on disk beforehand are kept.

Pass `--clean` (`ctg run --clean` or `ctgx --clean`) to ensure all decrypted files are cleaned up even if they were present beforehand.

```bash +exec
git checkout . -q && ctg clean -qq

# Run 'ls' while secrets are temporarily decrypted (auto-cleaned after command finishes)
ctg run -- ls ./secrets/secret.yaml
ctg run -- ls ./secrets/secret.yaml.cott.age
# Output:
# decrypt ./secrets/secret.yaml.cott.age
#    into ./secrets/secret.yaml
# ./secrets/secret.yaml
# decrypt ./secrets/secret.yaml.cott.age
#    into ./secrets/secret.yaml
# ./secrets/secret.yaml

# Pass --clean to ensure decrypted secrets are cleaned up even if already present
ctg run --clean -- ./deploy.sh
ctgx --clean ./deploy.sh
```

---

# `ctg env`

Decrypt an environment file in memory and export its content as environment variables for the specified command.

It defaults to `.env.cott.age` in the current directory.

```bash +exec
git checkout . -q && ctg clean -qq

# Run 'printenv' with variables from .env.cott.age
ctg env -F ./.env.cott.age -- printenv SECRET
# Output:
# foobar

# If the file is not a valid dotenv file, it exports the entire content as COTTAGE_SECRET
ctg env -F ./secrets/secret.yaml.cott.age -- sh -c 'echo "$COTTAGE_SECRET"'
# Output:
# SECRET: foobar
```

### Target Behavior

- **`-F`, `--file`**: Specifies the encrypted file to use. It must be an encrypted file (ending in `.cott.age`).

---

# Remote Upstreams

`ctg` can be configured to pull/push secrets from/to remote upstreams like HashiCorp Vault, AWS Secrets Manager, or custom APIs.

Upstreams are defined in `cottage.toml` and linked to secrets in their `.cott.toml` metadata files.

### Configuration Example (`cottage.toml`)

```toml
[upstream.customvault]
vars = { HOST = "customvault.example.com" }

[upstream.customvault.pull]
script = 'curl -s "https://${HOST}/api/pull${DESTINATION}"'

[upstream.customvault.push]
script = 'curl -s -X POST -d @- "https://${HOST}/api/push${DESTINATION}"'
```

### Linking a Secret (`./secrets/secret.json.cott.toml`)

```toml
[upstream.customvault]
pull = true
push = true
vars = {
  DESTINATION = "/vault/myapp/env/staging"
}
```

---

# Upstream Plugins

Instead of writing scripts directly in `cottage.toml`, you can use external binaries or scripts as plugins to manage interactions with your upstream.

See the [Python-based Kubernetes/Vault plugin example](./plugins/cottage-plugin-vault-in-kubernetes.py).

This plugin script:

- Forwards ports to connect to a Vault service inside a Kubernetes cluster.
- Pulls/pushes secrets via Vault's REST API.

To use a plugin, define `plugin` in the upstream configuration in `cottage.toml`:

```toml
[upstream.dev-vault]
plugin = "./examples/plugins/cottage-plugin-vault-in-kubernetes.py"
```

---

# `ctg pull`

Fetch secrets from a remote upstream and encrypt them locally.

```bash +exec
git checkout . -q && ctg clean -qq

# Pull a secret from the 'customvault' upstream
ctg pull customvault ./secrets/secret.json.cott.age
# Output:
# pull    customvault
#    into ./secrets/secret.json.cott.age
#    edit secrets/secret.json.cott.toml
```

### Target Behavior

- **`.cott.age` File**: Pulls the secret for this specific file if it has an upstream configured in its metadata.
- **Directory**: Recursively pulls secrets for all `.cott.age` files that have upstreams configured.
- **Decrypted File**: Pulls for the corresponding `.cott.age` file.

---

# `ctg push`

Push locally encrypted secrets (decrypted in memory) to a remote upstream.

```bash +exec
git checkout . -q && ctg clean -qq

# Push a secret to the 'customvault' upstream
ctg push customvault ./secrets/secret.json.cott.age
# Output:
# push    ./secrets/secret.json.cott.age
#    into customvault
```

### Target Behavior

- **`.cott.age` File**: Pushes the secret for this specific file if it has an upstream configured in its metadata.
- **Directory**: Recursively pushes secrets for all `.cott.age` files that have upstreams configured.
- **Decrypted File**: Pushes for the corresponding `.cott.age` file.

---

# Common Options

Many `ctg` commands share these common options:

- `-f`, `--force`: Skip checksum verification and force the operation (e.g., re-encrypt/re-decrypt even if timestamps match).
- `-n`, `--dry-run`: Show what would be done without actually making any changes.
- `-R`, `--recipients-file PATH`: Encrypt to or verify against recipients listed at PATH.
- `--skip-verify-encrypted`: Skip checksum verification of encrypted files.
- `--skip-verify-recipients`: Skip checksum verification of recipients.
- `--skip-preview`: Skip generation of previews for encrypted files.
- `--skip-timestamps`: Skip updating timestamps on files after encryption/decryption.
- `--skip-gitignore`: Skip adding files to `.gitignore`.
- `--skip-encryption`: Skip operations involving encryption (sync, diff, status).
- `--skip-decryption`: Skip operations involving decryption (sync, diff, status).

---

# Command Option Examples

```bash +exec
git checkout . -q && ctg clean -qq

ctg decrypt ./secrets/secret.yaml.cott.age --force
# Output:
# decrypt ./secrets/secret.yaml.cott.age
#    into ./secrets/secret.yaml
# (Even if the decrypted file is up-to-date, it will be re-decrypted)

# Dry run: see what would be decrypted
ctg decrypt ./secrets/secret.json.cott.age --dry-run
# Output:
# decrypt ./secrets/secret.json.cott.age
#    into ./secrets/secret.json


# Skip encryption when checking status
echo "change" >> ./secrets/secret.yaml
ctg status ./secrets/secret.yaml --skip-encryption
# (No output, as only pending encryption exists)
```

---

# `ctg autocomplete`

Generate shell completions for Bash, Zsh, Fish, etc.

```bash
# Generate and source Bash completions
echo 'eval "$(ctg autocomplete bash)"' >> "~/.bashrc"
source ~/.bashrc

# Generate and source Zsh completions
echo 'eval "$(ctg autocomplete zsh)"' >> "~/.zshrc"
source ~/.zshrc
```
