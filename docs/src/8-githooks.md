# Configuring Git Hooks

Git hooks can help you automate the encryption and decryption of secrets, ensuring that you never accidentally push unencrypted secrets or forget to decrypt them after pulling.

Scenarios for configuring git hooks:

1.  [I want to see secret diff before git push](#i-want-to-see-secret-diff-before-git-push)
2.  [I want to auto-sync secrets before git push and after git pull](#i-want-to-auto-sync-secrets-before-git-push-and-after-git-pull)

## I want to see secret diff before git push

You can use [prek](https://github.com/j178/prek) to automatically show the diff of secrets before pushing. This acts as a final check to ensure you are pushing exactly what you intend.

Add the following to your `prek.toml`:

```toml
[[repos]]
repo = "https://github.com/sayanarijit/cottage"
rev = "main"

[[repos.hooks]]
id = "cottage-diff"
```

Then install the hooks:

```bash
prek install
```

Now, every time you run `git push`, `prek` will run `ctg diff`. If there are any differences between your decrypted secrets and their encrypted counterparts, they will be displayed:

> ```bash
> git push origin main
> ```
>
> ```diff,stdout
> diff --git a/secret.env b/secret.env
> --- a/secret.env
> +++ b/secret.env
> @@ -1 +1 @@
> -DB_PASSWORD=old-password
> +DB_PASSWORD=new-password
> ```

## I want to auto-sync secrets before git push and after git pull

To ensure that your secrets are always in sync, you can set up hooks to automatically encrypt before committing and decrypt after pulling.

Add the following to your `prek.toml`:

```toml
[[repos]]
repo = "https://github.com/sayanarijit/cottage"
rev = "main"

# Automatically encrypt modified secrets
[[repos.hooks]]
id = "cottage-sync-encrypt"

# Automatically decrypt updated secrets
[[repos.hooks]]
id = "cottage-sync-decrypt"
```

Install the hooks:

```bash
prek install
```

With this setup:

- When you run `git commit`, any modified secrets will be automatically encrypted:

  > ```bash
  > git commit -m "Update secrets"
  > ```
  >
  > ```stdout
  > encrypt secret.env
  >    into secret.env.cott.age
  >    edit secret.env.cott.toml
  > [main XXXXXXX] Update secrets
  >  2 files changed, 2 insertions(+), 2 deletions(-)
  > ```

- When you run `git pull`, any updated `.cott.age` files will be automatically decrypted:

  > ```bash
  > git pull origin main
  > ```
  >
  > ```stdout
  > Updating XXXXXXX..XXXXXXX
  > Fast-forward
  >  secret.env.cott.age  | Bin XXX -> XXX bytes
  >  secret.env.cott.toml |   4 ++--
  > decrypt secret.env.cott.age
  >    into secret.env
  > ```
