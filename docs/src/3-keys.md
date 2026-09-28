# Setting up keys

Scenarios for configuring keys for a new or existing git repository:

1.  [I want to create a new key pair for a new project](#i-want-to-create-a-new-key-pair-for-a-new-project)
2.  [I want to use the same key across multiple projects](#i-want-to-use-the-same-key-across-multiple-projects)
3.  [I want to use my existing SSH key pair](#i-want-to-use-my-existing-ssh-key-pair)
4.  [I want to generate or re-generate a new key pair in an existing project](#i-want-to-generate-or-re-generate-a-new-key-pair-in-an-existing-project)

## I want to create a new key pair for a new project

> [!IMPORTANT]
> Starting from version 0.8, cottage generates and expects private keys outside of the project repository.

When you run `ctg init` or `ctg keygen`:

- The private key is stored in `~/.config/cottage/identity/<dirname>/<timestamp>.key` (where `<dirname>` is the project root directory name).
- The public key is stored in `.cottage/recipients/<username>`.

When searching for default private keys, cottage resolves keys in the following order:

1. If `~/.config/cottage/identity/<dirname>` is present, it loads all keys from that directory.
2. Otherwise, if the parent directory `~/.config/cottage/identity` is present, it loads all keys from it recursively.
3. Otherwise, if `~/.ssh` is present, it loads all keys from `~/.ssh`.

You can also pass explicit private key files or identity strings using the `-i / --identity` flag or the `COTTAGE_IDENTITY` environment variable.

## I want to use the same key across multiple projects

If you want to share a key across multiple projects, you can place the private key directly in the parent directory `~/.config/cottage/identity/` (e.g. `~/.config/cottage/identity/global.key`).

When a project doesn't have a project-specific directory `~/.config/cottage/identity/<dirname>`, cottage will automatically load keys from `~/.config/cottage/identity/` recursively.

## I want to use my existing SSH key pair

> [!WARNING]
> While `cottage` supports using your existing SSH keys (e.g., the ones you use for Git authentication), it is highly recommended to maintain separation between keys used for different scopes and purposes.

If you already have an SSH key pair[^keypair], you can use it with cottage by copying your public key to the `.cottage/recipients` directory:

> [^keypair]: (cott)age is compatible with RSA and Ed25519 keys that are generated without passphrase. You can always generate a new SSH (e.g. RSA) key using `ssh-keygen` (e.g. `ssh-keygen -t rsa`) to use with cottage.

> ```bash
> cp -v ~/.ssh/id_rsa.pub .cottage/recipients/$USER
> ```

If no project-specific identity directory `~/.config/cottage/identity/<dirname>` or global `~/.config/cottage/identity` exists, cottage will automatically fall back to loading keys from `~/.ssh/`. Alternatively, you can explicitly point to your SSH private key using `-i ~/.ssh/id_rsa`.

## I want to generate or re-generate a new key pair in an existing project

If you are setting up keys in an existing cottage project, or want to re-generate existing keys, you can run the `ctg keygen` command:

> ```bash
> ctg keygen
> ```

By default, this generates a key pair where the recipient public key file is named after your system username (i.e., `$USER`), and the private key is placed in `~/.config/cottage/identity/<dirname>/<timestamp>.key`.

You can customize the name of the public key file in `.cottage/recipients/` using the `-n` or `--name` option:

> ```bash
> ctg keygen -n myname
> ```

To force re-generation of the key pair and overwrite any existing keys for the project, use the `--force` option:

> ```bash
> ctg keygen -n myname --force
> ```
