---
title: "cottage (ctg)"
sub_title: "Modern git-based age-encrypted secrets manager"
author: "Sayanarijit"
theme:
  name: dark
---

# cottage

<!-- jump_to_middle -->

`ctg` (cottage) is a modern git-based age-encrypted secrets manager for teams.

This guide covers all subcommands and their options with real examples.

This document is best viewed in a terminal with [presenterm](https://github.com/mfontanini/presenterm/).

> [!NOTE]
> The term _"tracked secret"_ or just _"secret"_ used in this guide refers to secrets that have been encrypted, i.e. has a corresponding `.cott.age` file.

<!-- end_slide -->

# Path Target Behavior

Most `ctg` commands can take a file or a directory as an argument.

- **File**: Operates on that specific file.
- **Directory**: Recursively operates on all tracked secrets within that directory.
- **`.cott.age` file**: Usually treated as the source for decryption or the target for status/diff.
- **`.cott.toml` file**: Metadata file. Most commands skip these directly as they are managed alongside the `.cott.age` files.

<!-- end_slide -->

# `ctg init`

Initialize cottage in the current directory. This creates a `.cottage` directory for recipients and identities.

```bash
git checkout examples && ctg clean -qq

ctg init
# (Initializes .cottage directory)
```

> [!NOTE]
> This command is optional in git projects. In non-git projects, typically run once at the start of a project.

<!-- end_slide -->

# `ctg encrypt`

Encrypt files or directories. By default, it processes all the tracked secrets the entire project root.

```bash
git checkout examples && ctg clean -qq

# First, unmark a secret by deleting the .cott.age file
rm examples/secrets/secret.yaml.cott.*

echo "added: line" >> examples/secrets/secret.yaml

# Now encrypt the file and start tracking it again
ctg encrypt examples/secrets/secret.yaml
# Output:
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
```

### Target Behavior

- **Decrypted File**: Encrypts it into a `.cott.age` file and updates metadata.
- **Directory**: Recursively finds and encrypts all decrypted secrets.
- **`.cott.age`**: Skipped (already encrypted).
- **`.cott.toml`**: Skipped (metadata).

<!-- end_slide -->

# `ctg decrypt`

Decrypt files or directories.

```bash
git checkout examples && ctg clean -qq

# Decrypt a specific secret
ctg decrypt examples/secrets/secret.yaml.cott.age
# Output:
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml
```

### Target Behavior

- **`.cott.age` File**: Decrypts it into the corresponding plain-text file.
- **Directory**: Recursively finds and decrypts all `.cott.age` files.
- **Decrypted File**: Skipped (already decrypted).
- **`.cott.toml`**: Skipped (metadata).

<!-- end_slide -->

# `ctg status`

See pending actions based on timestamps.

```bash
git checkout examples && ctg clean -qq

# Check status
ctg status examples/secrets/secret.yaml.cott.age
# Output:
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml

# Decrypt and modify to see pending encryption
ctg decrypt examples/secrets/secret.yaml.cott.age -qq
echo "status: change" >> examples/secrets/secret.yaml

ctg status examples/secrets/secret.yaml
# Output:
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
```

### Target Behavior

- **Any Target**: Works on any path that is part of a secret (plain-text or `.cott.age`) or a directory containing them.

<!-- end_slide -->

# `ctg diff`

See the actual diff between encrypted and decrypted files. This decrypts the encrypted version in memory (safe from accidental exposure) to compare.

```bash
git checkout examples && ctg clean -qq

# Decrypt and modify a file
ctg decrypt examples/secrets/secret.yaml.cott.age -qq
echo "diff: change" >> examples/secrets/secret.yaml

# View the diff
ctg diff examples/secrets/secret.yaml
```

Output:

```diff
diff --git a/examples/secrets/secret.yaml b/examples/secrets/secret.yaml
--- a/examples/secrets/secret.yaml
+++ b/examples/secrets/secret.yaml
@@ -1 +1,2 @@
 SECRET: foobar
+diff: change
```

### Target Behavior

- **Any Target**: Similar to `status`, it can be pointed at any file in a secret pair or a directory. It will decrypt the `.cott.age` file in memory and compare it with the on-disk plain-text file.

<!-- end_slide -->

# `ctg sync`

Keeps encrypted and decrypted files in sync based on timestamps and checksums.

```bash
git checkout examples && ctg clean -qq

# Decrypt and modify
ctg decrypt examples/secrets/secret.yaml.cott.age -qq
echo "sync: change" >> examples/secrets/secret.yaml

# Sync will encrypt the newer decrypted file
ctg sync examples/secrets/secret.yaml
# Output:
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
```

### Target Behavior

- **File/Dir/Age**: Syncs the target(s). If a decrypted file is newer, it encrypts. If a `.cott.age` file is newer, it decrypts.

<!-- end_slide -->

# `ctg edit`

Edit and encrypt a file directly. Opens it in your default `$EDITOR`, and re-encrypts it upon saving and exiting.

Run it with `--clean` to automatically delete the decrypted file after editing.

```bash
git checkout examples && ctg clean -qq

ctg edit examples/secrets/secret.yaml.cott.age --clean
# (Opens $EDITOR, then encrypts on save)
# Output:
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
# delete examples/secrets/secret.yaml
```

### Target Behavior

- **Plain Text**: Directly opens the decrypted file for editing, then encrypts it after saving.
- **`.cott.age`**: Decrypts for editing, then re-encrypts after saving.
- **Directory**: Not supported (must target a specific secret).
- **`.cott.toml`**: Not supported (cannot edit metadata directly).

<!-- end_slide -->

# `ctg clean`

Delete all decrypted secrets to keep the workspace clean.

```bash
git checkout examples && ctg clean -qq

# First, decrypt some secrets
ctg decrypt examples -qq

# Actually delete decrypted secrets
ctg clean examples
# Output:
# delete examples/secrets/secret.yaml
```

### Target Behavior

- **Decrypted File**: Deletes it.
- **Directory**: Recursively deletes all decrypted secrets within.
- **`.cott.age` / `.cott.toml`**: Skipped.

> [!WARNING] > `ctg clean` is destructive for your local decrypted copies. Always ensure your changes are encrypted before cleaning.

<!-- end_slide -->

# `ctg run` / `ctgx`

Decrypt secrets, run a specified command, and automatically delete the decrypted secrets after the command finishes.

```bash
git checkout examples && ctg clean -qq

# Run 'ls' while secrets are temporarily decrypted
ctg run -- ls examples/secrets/secret.yaml
# Output:
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml
# examples/secrets/secret.yaml
# delete examples/secrets/secret.yaml
```

### Target Behavior

- **Any Path**: While `run` takes a command, it uses the paths within the project to determine what to decrypt. By default, it decrypts _everything_ in the project root.

<!-- end_slide -->

# Common Options

Many `ctg` commands share these common options:

- `-f`, `--force`: Skip checksum verification and force the operation (e.g., re-encrypt/re-decrypt even if timestamps match).
- `-n`, `--dry-run`: Show what would be done without actually making any changes.
- `--skip-verify-encrypted`: Skip checksum verification of encrypted (`.cott.age`) files.
- `--skip-verify-decrypted`: Skip checksum verification of decrypted files.
- `--skip-preview`: Skip generation of previews for encrypted files.
- `--skip-timestamps`: Skip updating timestamps on files after encryption/decryption.
- `--skip-gitignore`: Skip adding files to `.gitignore`.
- `--skip-encryption`: Skip operations involving encryption (sync, diff, status).
- `--skip-decryption`: Skip operations involving decryption (sync, diff, status).

<!-- end_slide -->

### Command Option Examples

```bash
# Dry run: see what would be decrypted
ctg decrypt examples/secrets/secret.yaml.cott.age --dry-run
# Output:
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml

# Force re-encryption even if files are in sync
ctg encrypt examples/secrets/secret.yaml --force
# Output:
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml

# Skip encryption when checking status
echo "change" >> examples/secrets/secret.yaml
ctg status examples/secrets/secret.yaml --skip-encryption
# (No output, as only pending encryption exists)
```

<!-- end_slide -->

# `ctg autocomplete`

Generate shell completions for Bash, Zsh, Fish, etc.

```bash
# Generate and source Bash completions
echo 'eval "$(ctg autocomplete bash)"' >> "~/.basrc"
source ~/.bashrc

# Generate and source Zsh completions
echo 'eval "$(ctg autocomplete zsh)"' >> "~/.zshrc"
source ~/.zshrc
```
