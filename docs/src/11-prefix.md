# Path Prefixes

Scenarios for keeping encrypted secrets and metadata out of your source tree:

1.  [I want to store encrypted secrets and metadata in a separate directory](#i-want-to-store-encrypted-secrets-and-metadata-in-a-separate-directory)
2.  [I want to move existing secrets to the prefixed directory](#i-want-to-move-existing-secrets-to-the-prefixed-directory)
3.  [I want to stop using prefixes](#i-want-to-stop-using-prefixes)

## I want to store encrypted secrets and metadata in a separate directory

By default, `secret.txt.cott.age` and `secret.txt.cott.toml` live right next to `secret.txt`.
To keep them in dedicated directories instead, declare `path.prefix` in `cottage.toml`:

```toml
[path.prefix]
encrypted = "./.cottage/secrets" # where *.cott.age files are stored
metadata = "./.cottage/secrets"  # where *.cott.toml files are stored
```

Either field can be omitted. Relative prefixes are resolved against the project root.

With the declaration above, cottage reads and writes `sub/secret.txt` as:

```
.cottage/secrets/sub/secret.txt.cott.age
.cottage/secrets/sub/secret.txt.cott.toml
```

The prefixed directories are generated automatically. All the usual commands (`encrypt`, `decrypt`, `edit`, `status`, `sync`, `run`, ...) keep working with the decrypted path.

## I want to move existing secrets to the prefixed directory

After adding `path.prefix` to an existing project, the old `*.cott.age` and `*.cott.toml` files are "strays" next to their decrypted counterparts.
Move them to the prefixed paths with:

```bash
ctg prefix sync            # the whole project
ctg prefix sync ./sub      # only a file or directory
ctg prefix sync -n         # dry run: show what would be moved
```

`ctg prefix sync` refuses to overwrite a file that already exists at the destination.

## I want to stop using prefixes

Move the secrets and metadata back to their unprefixed relative paths **before** removing the declaration from `cottage.toml`:

```bash
ctg prefix revert          # use -n for a dry run
```

Then remove the `[path.prefix]` section from `cottage.toml`.
