# Example secrets

`ctg` (cottage) is a modern git-based age-encrypted secrets manager for teams. This guide covers all subcommands and their options with real examples.

## Path Target Behavior

Most `ctg` commands can take a file or a directory as an argument.
- **File**: Operates on that specific file.
- **Directory**: Recursively operates on all secrets within that directory.
- **`.cott.age` file**: Usually treated as the source for decryption or the target for status/diff.
- **`.cott.toml` file**: Metadata file. Most commands skip these directly as they are managed alongside the `.cott.age` files.

---

## `ctg init`

Initialize cottage in the current directory. This creates a `.cottage` directory for recipients and identities.

```bash
git checkout examples && ctg clean -qq
ctg init
# (Initializes .cottage directory)
```

> [!NOTE]
> This command is typically run once at the start of a project.

---

## `ctg encrypt`

Encrypt files or directories. By default, it processes the entire project root.

```bash
git checkout examples && ctg clean -qq

# First, decrypt a file to modify it
ctg decrypt examples/secrets/secret.yaml.cott.age
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml

echo "added: line" >> examples/secrets/secret.yaml

# Now encrypt the modified file
ctg encrypt examples/secrets/secret.yaml
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
```

### Target Behavior
- **Decrypted File**: Encrypts it into a `.cott.age` file and updates metadata.
- **Directory**: Recursively finds and encrypts all decrypted secrets.
- **`.cott.age`**: Skipped (already encrypted).
- **`.cott.toml`**: Skipped (metadata).

---

## `ctg decrypt`

Decrypt files or directories.

```bash
git checkout examples && ctg clean -qq

# Decrypt a specific secret
ctg decrypt examples/secrets/secret.yaml.cott.age
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml
```

### Target Behavior
- **`.cott.age` File**: Decrypts it into the corresponding plain-text file.
- **Directory**: Recursively finds and decrypts all `.cott.age` files.
- **Decrypted File**: Skipped (already decrypted).
- **`.cott.toml`**: Skipped (metadata).

---

## `ctg status`

See pending actions based on timestamps.

```bash
git checkout examples && ctg clean -qq

# Check status (shows pending decryption by default)
ctg status examples/secrets/secret.yaml.cott.age
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml

# Decrypt and modify to see pending encryption
ctg decrypt examples/secrets/secret.yaml.cott.age -q
echo "status: change" >> examples/secrets/secret.yaml
ctg status examples/secrets/secret.yaml
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
```

### Target Behavior
- **Any Target**: Works on any path that is part of a secret (plain-text, `.cott.age`, or `.cott.toml`) or a directory containing them. It reports whether encryption or decryption is needed to bring them in sync.

---

## `ctg diff`

See the actual diff between encrypted and decrypted files. This decrypts the encrypted version in memory to compare.

```bash
git checkout examples && ctg clean -qq

# Decrypt and modify a file
ctg decrypt examples/secrets/secret.yaml.cott.age -q
echo "diff: change" >> examples/secrets/secret.yaml

# View the diff
ctg diff examples/secrets/secret.yaml
# diff --git a/examples/secrets/secret.yaml b/examples/secrets/secret.yaml
# --- a/examples/secrets/secret.yaml
# +++ b/examples/secrets/secret.yaml
# @@ -1 +1,2 @@
#  SECRET: foobar
# +diff: change
```

### Target Behavior
- **Any Target**: Similar to `status`, it can be pointed at any file in a secret pair or a directory. It will decrypt the `.cott.age` file in memory and compare it with the on-disk plain-text file.

---

## `ctg sync`

Keeps encrypted and decrypted files in sync based on timestamps and checksums.

```bash
git checkout examples && ctg clean -qq

# Decrypt and modify
ctg decrypt examples/secrets/secret.yaml.cott.age -q
echo "sync: change" >> examples/secrets/secret.yaml

# Sync will encrypt the newer decrypted file
ctg sync examples/secrets/secret.yaml
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
```

### Target Behavior
- **File/Dir/Age**: Syncs the target(s). If a decrypted file is newer, it encrypts. If a `.cott.age` file is newer, it decrypts.

---

## `ctg edit`

Edit an encrypted file directly. It decrypts the file, opens it in your default `$EDITOR`, and re-encrypts it upon saving and exiting.

```bash
git checkout examples && ctg clean -qq

ctg edit examples/secrets/secret.yaml.cott.age
# (Opens $EDITOR, then encrypts on save)
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
```

### Target Behavior
- **File or `.cott.age`**: Both work. It identifies the secret pair and opens the decrypted version (creating it if necessary).
- **Directory**: Not supported (must target a specific secret).
- **`.cott.toml`**: Not supported (cannot edit metadata directly).

---

## `ctg clean`

Delete all decrypted secrets to keep the workspace clean.

```bash
git checkout examples && ctg clean -qq

# First, decrypt some secrets
ctg decrypt examples -q

# Dry run to see what would be deleted
ctg clean examples --dry-run
# delete examples/secrets/secret.yaml
# ...

# Actually delete decrypted secrets
ctg clean examples
# delete examples/secrets/secret.yaml
# ...
```

### Target Behavior
- **Decrypted File**: Deletes it.
- **Directory**: Recursively deletes all decrypted secrets within.
- **`.cott.age` / `.cott.toml`**: Skipped (they are the "source of truth" and not cleaned).

> [!WARNING]
> `ctg clean` is destructive for your local decrypted copies. Always ensure your changes are encrypted (`ctg encrypt`) before cleaning.

---

## `ctg run` / `ctgx`

Decrypt secrets, run a specified command, and automatically delete the decrypted secrets after the command finishes.

```bash
git checkout examples && ctg clean -qq

# Run 'ls' while secrets are temporarily decrypted
ctg run -- ls examples/secrets/secret.yaml
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml
# examples/secrets/secret.yaml
# delete examples/secrets/secret.yaml
```

### Target Behavior
- **Any Path**: While `run` takes a command, it uses the paths within the project to determine what to decrypt. If you provide specific paths via environment variables or CLI flags (not shown here but supported by `ctg`), it would focus on those. By default, it decrypts *everything* in the project root to ensure the command has access to all secrets.

---

## `ctg autocomplete`

Generate shell completions for Bash, Zsh, Fish, etc.

```bash
ctg autocomplete bash
# _ctg() {
#     local i cur prev opts cmd
#     COMPREPLY=()
# ...
```
