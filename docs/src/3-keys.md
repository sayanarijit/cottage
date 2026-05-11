# Setting up keys

Scenarios for configuring keys for a new or existing git repository:

1.  [I want to use the auto-generated key pair across multiple projects](#i-want-to-use-the-auto-generated-key-pair-across-multiple-projects)
2.  [I want to use my existing SSH key pair](#i-want-to-use-my-existing-ssh-key-pair)
3.  [I want to avoid symlinking my private key in the workspace](#i-want-to-avoid-symlinking-my-private-key-in-the-workspace)

## I want to use the auto-generated key pair across multiple projects

[ctg init](2-init.md) auto generates a new key for convenience. You can (but don't have to) use the same key across multiple projects.

To do that, you can copy the private key to `~/.cottage/identity/` and symlink it back to the project:

> ```bash,test
> mkdir -p ~/.config/cottage/identity
> chmod 700 ~/.config/cottage/identity
> mv -v .cottage/identity ~/.config/cottage/identity/"$(basename $PWD)"
> ln -s -v ~/.config/cottage/"$(basename $PWD)" .cottage/identity
> ```
>
> ```stdout
> renamed '.cottage/identity' -> '/home/sayanarijit/.config/cottage/identity/tmp.vehzl30boG'
> '.cottage/identity' -> '/home/sayanarijit/.config/cottage/tmp.vehzl30boG'
> ```

## I want to use my existing SSH key pair

If you already have an SSH key pair[^keypair] (e.g. the one you use with git), you can use it with cottage by adding a symlink to the private key in the `.cottage/identity` file or directory, and copying the public key to the `.cottage/recipients` directory.

> [^keypair]: (cott)age is compatible with RSA and Ed25519 keys that are generated without passphrase. You can always generate a new SSH (e.g. RSA) key using `ssh-keygen` (e.g. `ssh-keygen -t rsa`) to use with cottage.

> ```bash,test
> # ssh-keygen -t rsa  # (optional: generate a new RSA key pair without passphrase)
> rm -v .cottage/identity
> ln -s -v ~/.ssh/id_rsa .cottage/identity
> cp -v ~/.ssh/id_rsa.pub .cottage/recipients/$USER
> ```
>
> ```stdout
> removed '.cottage/identity'
> '.cottage/identity' -> '/home/sayanarijit/.ssh/id_rsa'
> '/home/sayanarijit/.ssh/id_rsa.pub' -> '.cottage/recipients/sayanarijit'
> ```

## I want to avoid symlinking my private key in the workspace

You don't have to symlink or copy your private key in the workspace.

By default, cottage looks for private keys in the `.cottage/identity` file or directory.

If the directory is absent, it will try to load all keys from `~/.cottage/identity`.

If the directory is absent, it will try to load all keys from `~/.ssh`.

You can also always mention the path to the private key using the `-i / --identity` flag or the `COTTAGE_IDENTITY` environment variable.

> ```bash,test
> rm -v .cottage/identity
> ```
>
> ```
> removed '.cottage/identity'
> ```
