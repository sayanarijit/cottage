# Configuring Keys

Scenarios for configuring keys for a new or existing git repository:

## Start using the auto-generated key pair accross projects

[ctg init](2-init.md) auto generates a new key for convenience. You can (but don't have to) use the same key across multiple projects.

To do that, You can copy the private key to `$HOME/.cottage/identity/` and symlink it back to the project:

> ```bash,test
> mkdir -p ~/.config/cottage/identity
> chmod 700 ~/.config/cottage/identity
> mv -v ".cottage/identity" "~/.config/cottage/identity/$(basename $PWD)"
> ln -s -v "~/.config/cottage/$(basename $PWD)" ".cottage/identity"
> ```
>
> ```stdout
> renamed '.cottage/identity' -> '/home/sayanarijit/.config/cottage/identity/tmp.vehzl30boG'
> '.cottage/identity' -> '/home/sayanarijit/.config/cottage/tmp.vehzl30boG'
> ```

## Use your existing SSH RSA/Ed25519 key pair

If you already have an SSH key pair[^note] (e.g. the one you use with git), you can use it with cottage by adding a symlink to the private key in the `.cottage/identity` directory, and copying the public key to the `.cottage/recipients` directory.

> [^note]: (cott)age is compatible with RSA and Ed25519 keys that are geneated without passphrase. You can always generate a new SSH (e.g. RSA) key using `ssh-keygen` (e.g. `ssh-keygen -t rsa`) to use with cottage.

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
