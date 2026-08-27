# Setting up keys

Scenarios for configuring keys for a new or existing git repository:

1.  [I want to use the auto-generated key pair across multiple projects](#i-want-to-use-the-auto-generated-key-pair-across-multiple-projects)
2.  [I want to use my existing SSH key pair](#i-want-to-use-my-existing-ssh-key-pair)
3.  [I want to avoid symlinking my private key in the workspace](#i-want-to-avoid-symlinking-my-private-key-in-the-workspace)
4.  [I want to generate or re-generate a new key pair in an existing project](#i-want-to-generate-or-re-generate-a-new-key-pair-in-an-existing-project)

## I want to use the auto-generated key pair across multiple projects

[ctg init](2-init.md) auto generates a new key for convenience. You can (but don't have to) use the same key across multiple projects.

To do that, you can copy the private key to `~/.config/cottage/identity/` and symlink it back to the project:

> ```bash,test,session=myproject:4
> mkdir -p ~/.config/cottage/identity
> chmod 700 ~/.config/cottage/identity
> mv -v .cottage/identity ~/.config/cottage/identity/"$(basename $PWD)"
> ln -s -v ~/.config/cottage/identity/"$(basename $PWD)" .cottage/identity
> ```
>
> ```stdout
> renamed '.cottage/identity' -> '/home/...XXX.../.config/cottage/identity/tmp....XXX...'
> '.cottage/identity' -> '/home/...XXX.../.config/cottage/identity/tmp....XXX...'
> ```

## I want to use my existing SSH key pair

> [!WARNING]
> While `cottage` supports using your existing SSH keys (e.g., the ones you use for Git authentication), it is highly recommended to maintain separation between keys used for different scopes and purposes.

If you already have an SSH key pair[^keypair], you can use it with cottage by adding a symlink to the private key in the `.cottage/identity` file or directory, and copying the public key to the `.cottage/recipients` directory.

> [^keypair]: (cott)age is compatible with RSA and Ed25519 keys that are generated without passphrase. You can always generate a new SSH (e.g. RSA) key using `ssh-keygen` (e.g. `ssh-keygen -t rsa`) to use with cottage.

> ```bash,test,session=myproject:5
> # ssh-keygen -t rsa  # (optional: generate a new RSA key pair without passphrase)
> rm -v .cottage/identity
> ln -s -v ~/.ssh/id_rsa .cottage/identity
> cp -v ~/.ssh/id_rsa.pub .cottage/recipients/$USER
> ```
>
> ```stdout
> removed '.cottage/identity'
> '.cottage/identity' -> '/home/...XXX.../.ssh/id_rsa'
> '/home/...XXX.../.ssh/id_rsa.pub' -> '.cottage/recipients/...XXX...'
> ```

## I want to avoid symlinking my private key in the workspace

You don't have to symlink or copy your private key in the workspace.

By default, cottage looks for private keys in the `.cottage/identity` file or directory.

If the project-level identity is absent, it will try to load all keys from `~/.config/cottage/identity`.

If that is also absent, it will try to load all keys from `~/.ssh`.

You can also always pass the path to the private key, or the identity string itself, using the `-i / --identity` flag or the `COTTAGE_IDENTITY` environment variable.

> ```bash
> rm -v .cottage/identity
> ```
>
> ```stdout
> removed '.cottage/identity'
> ```

## I want to generate or re-generate a new key pair in an existing project

If you are setting up keys in an existing cottage project, or want to re-generate existing keys, you can run the `ctg keygen` command:

> ```bash
> ctg keygen
> ```

By default, this generates a key pair where the recipient public key file is named after your system username (i.e., `$USER`).

You can customize the name of the public key file in `.cottage/recipients/` using the `-n` or `--name` option:

> ```bash
> ctg keygen -n myname
> ```

To force re-generation of the key pair and overwrite any existing identity file, use the `--force` option:

> ```bash
> ctg keygen -n myname --force
> ```
