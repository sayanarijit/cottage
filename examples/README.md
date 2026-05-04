# Example secrets

### `ctg run` / `ctgx`

```bash
# Regular tree output
tree examples
# examples
# ├── README.md
# └── secrets
#     ├── secret.hcl.cott.age
#     ├── secret.hcl.cott.toml
#     ├── secret.ini.cott.age
#     ├── secret.ini.cott.toml
#     ├── secret.json.cott.age
#     ├── secret.json.cott.toml
#     ├── secret.toml.cott.age
#     ├── secret.toml.cott.toml
#     ├── secret.yaml.cott.age
#     └── secret.yaml.cott.toml
#
# 2 directories, 11 files

# Run tree with cottage wrapper
ctg run -qq tree examples
# examples
# ├── README.md
# └── secrets
#     ├── secret.hcl
#     ├── secret.hcl.cott.age
#     ├── secret.hcl.cott.toml
#     ├── secret.ini
#     ├── secret.ini.cott.age
#     ├── secret.ini.cott.toml
#     ├── secret.json
#     ├── secret.json.cott.age
#     ├── secret.json.cott.toml
#     ├── secret.toml
#     ├── secret.toml.cott.age
#     ├── secret.toml.cott.toml
#     ├── secret.yaml
#     ├── secret.yaml.cott.age
#     └── secret.yaml.cott.toml
#
# 2 directories, 16 files
```

### `ctg decrypt`

```bash
# Single command to decrypt all secrets in the examples directory
ctg decrypt examples
# decrypt examples/.env.cott.age
#    into examples/.env
# decrypt examples/secrets/secret.json.cott.age
#    into examples/secrets/secret.json
# decrypt examples/secrets/secret.hcl.cott.age
#    into examples/secrets/secret.hcl
# decrypt examples/secrets/secret.toml.cott.age
#    into examples/secrets/secret.toml
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml
# decrypt examples/secrets/secret.ini.cott.age
#    into examples/secrets/secret.ini

echo 'foo: bar' | tee -a examples/secrets/secret.yaml
rm -v examples/secrets/secret.hcl
# foo: bar
# removed 'examples/secrets/secret.hcl'

# Dcrypts only what is missing or altered
ctg decrypt examples
# decrypt examples/secrets/secret.hcl.cott.age
#    into examples/secrets/secret.hcl
# decrypt examples/secrets/secret.yaml.cott.age
#    into examples/secrets/secret.yaml

ctg decrypt examples # Nothing to decrypt now
```

### `ctg encrypt`

```bash
echo 'foo: bar' | tee -a examples/secrets/secret.yaml
# foo: bar

# Encrypts only what is altered
ctg encrypt examples
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml

# Forces encryption of all secrets
ctg encrypt examples -f
# encrypt examples/.env
#    into examples/.env.cott.age
#    edit examples/.env.cott.toml
# encrypt examples/secrets/secret.json
#    into examples/secrets/secret.json.cott.age
#    edit examples/secrets/secret.json.cott.toml
# encrypt examples/secrets/secret.toml
#    into examples/secrets/secret.toml.cott.age
#    edit examples/secrets/secret.toml.cott.toml
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
# encrypt examples/secrets/secret.ini
#    into examples/secrets/secret.ini.cott.age
#    edit examples/secrets/secret.ini.cott.toml

# Encrypt and delete decrypted secrets
ctg encrypt examples --clean
# encrypt examples/.env
#    into examples/.env.cott.age
#    edit examples/.env.cott.toml
# delete examples/.env
# ...
```

### `ctg status`

```bash
# See which secrets need encryption or decryption
ctg status examples
# decrypt examples/.env.cott.age
#    into examples/.env
# decrypt examples/secrets/secret.json.cott.age
#    into examples/secrets/secret.json
# ...
```

### `ctg diff`

```bash
# See changes in secrets without manually decrypting
echo 'NEW_SECRET: true' >> examples/secrets/secret.yaml

ctg diff examples/secrets/secret.yaml
```

```diff
diff --git a/examples/secrets/secret.yaml b/examples/secrets/secret.yaml
--- a/examples/secrets/secret.yaml
+++ b/examples/secrets/secret.yaml
@@ -1 +1,4 @@
 SECRET: foobar
+NEW_SECRET: true
```

### `ctg sync`

```bash
# Keeps encrypted and decrypted files in sync based on timestamps
ctg sync examples
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
```

### `ctg clean`

```bash
# Remove all decrypted secrets to keep the workspace clean
ctg clean examples
# delete examples/.env
# delete examples/secrets/secret.json
# delete examples/secrets/secret.hcl
# delete examples/secrets/secret.toml
# delete examples/secrets/secret.yaml
# delete examples/secrets/secret.ini

# Dry run to see what would be deleted
ctg clean examples --dry-run
# delete examples/.env
# delete examples/secrets/secret.json
# ...
```

### `ctg edit`

```bash
# Edit a secret directly (decrypts, opens editor, and re-encrypts on save)
ctg edit examples/secrets/secret.yaml
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml

# Edit and delete decrypted secret after saving
ctg edit examples/secrets/secret.yaml --clean
# encrypt examples/secrets/secret.yaml
#    into examples/secrets/secret.yaml.cott.age
#    edit examples/secrets/secret.yaml.cott.toml
# delete examples/secrets/secret.yaml
```
