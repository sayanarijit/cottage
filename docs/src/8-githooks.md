# Configuring Git Hooks

Git hooks can help you automate the encryption and decryption of secrets, ensuring that you never accidentally push unencrypted secrets or forget to decrypt them after pulling.

Scenarios for configuring git hooks:

1.  [I want to see secret diff before git commit](#i-want-to-see-secret-diff-before-git-commit)
2.  [I want to auto-sync secrets before git commit and after git pull](#i-want-to-auto-sync-secrets-before-git-commit-and-after-git-pull)

## I want to see secret diff before git commit

You can use [prek](https://github.com/j178/prek) to automatically show the diff of secrets before pushing. This acts as a final check to ensure you are pushing exactly what you intend.

Add the following to your `prek.toml` in your project root:

> ```bash,test,session=myproject:32
> cd /tmp/myproject
> git pull origin main
>
> cat > prek.toml <<EOF
> [[repos]]
> repo = "https://github.com/sayanarijit/cottage"
> rev = "main"
>
> [[repos.hooks]]
> id = "cottage-diff"
> EOF
>
> prek auto-update
> prek install
> prek install-hooks
> ```
>
> ```stdout
> remote: Enumerating objects: 7, done.
> remote: Counting objects: 100% (7/7), done.
> remote: Compressing objects: 100% (4/4), done.
> remote: Total 4 (delta 2), reused 0 (delta 0), pack-reused 0 (from 0)
> Unpacking objects: 100% (4/4), X.XX KiB | X.XX MiB/s, done.
> From /tmp/upstream
>  * branch            main       -> FETCH_HEAD
>    XXXXXXX..XXXXXXX  main       -> origin/main
> Updating deb8a9a..b72a974
> Fast-forward
>  secret1.env.cott.age  | Bin 1134 -> 1161 bytes
>  secret1.env.cott.toml |   6 +++---
>  2 files changed, 3 insertions(+), 3 deletions(-)
> warning: The following repos have mutable `rev` fields (moving tag / branch):
> https://github.com/sayanarijit/cottage: main
> Mutable references are never updated after first install and are not supported.
> See https://pre-commit.com/#using-the-latest-version-for-a-repository for more details.
> hint: `prek auto-update` often fixes this",
>
> https://github.com/sayanarijit/cottage
>   updating rev `main` -> `v0.5.2`
> prek installed at `.git/hooks/pre-commit`
> ```

Now, every time you run `git commit`, `prek` will run `ctg diff`.

If there are any differences between your decrypted secrets and their encrypted counterparts, they will be displayed:

> ```bash,test,session=myproject:33
> # Edit a secret without syncing
> echo "DB_PASSWORD=new-password" > secret1.env
> git add .
> git commit --allow-empty -m "Test Commit"
> ```
>
> ```diff,stdout
> diff --git a/secret1.env b/secret1.env
> --- a/secret1.env
> +++ b/secret1.env
> @@ -1 +1 @@
> -DB_PASSWORD=my-local-password
> +DB_PASSWORD=new-password
> ```

To avoid this, you can run `ctg sync` to encrypt the modified secrets before committing:

> ```bash,test,session=myproject:34
> ctg sync
>
> git add .
> git commit -m "Update secrets"
> git push origin main
> ```
>
> ```stdout
> encrypt secret1.env
>    into secret1.env.cott.age
>    edit secret1.env.cott.toml
> cottage-diff.............................................................Passed
> [main XXXXXXX] Updated secrets
> 3 files changed, 9 insertions(+), 3 deletions(-)
> create mode 100644 prek.toml
> Enumerating objects: 8, done.
> Counting objects: 100% (8/8), done.
> Delta compression using up to 20 threads
> Compressing objects: 100% (5/5), done.
> Writing objects: 100% (5/5), X.XX KiB | X.XX MiB/s, done.
> Total 5 (delta 1), reused 0 (delta 0), pack-reused 0 (from 0)
> To /tmp/upstream.git
>    XXXXXXX..XXXXXXX  main -> main
> ```

## I want to auto-sync secrets before git commit and after git pull

To ensure that your secrets are always in sync, you can set up hooks to automatically encrypt before committing and decrypt after pulling.

Update your `prek.toml`:

> ```bash,test,session=myproject:35
> cd /tmp/myproject-clone
> git pull origin main
>
> cat > prek.toml <<EOF
> [[repos]]
> repo = "https://github.com/sayanarijit/cottage"
> rev = "main"
>
> # Automatically encrypt modified secrets
> [[repos.hooks]]
> id = "cottage-sync-encrypt"
>
> # Automatically decrypt updated secrets
> [[repos.hooks]]
> id = "cottage-sync-decrypt"
> EOF
>
> prek auto-update
> prek install
> prek install-hooks
> ```
>
> ```stdout
> remote: Enumerating objects: 8, done.
> remote: Counting objects: 100% (8/8), done.
> remote: Compressing objects: 100% (5/5), done.
> remote: Total 5 (delta 1), reused 0 (delta 0), pack-reused 0 (from 0)
> Unpacking objects: 100% (5/5), X.XX KiB | X.XX MiB/s, done.
> From /tmp/upstream
>  * branch            main       -> FETCH_HEAD
>    XXXXXXX..XXXXXXX  main       -> origin/main
> Updating XXXXXXX..XXXXXXX
> Fast-forward
>  prek.toml             |   6 ++++++
>  secret1.env.cott.age  | Bin 1161 -> 1077 bytes
>  secret1.env.cott.toml |   6 +++---
>  3 files changed, 9 insertions(+), 3 deletions(-)
>  create mode 100644 prek.toml
> warning: The following repos have mutable `rev` fields (moving tag / branch):
> https://github.com/sayanarijit/cottage: main
> Mutable references are never updated after first install and are not supported.
> See https://pre-commit.com/#using-the-latest-version-for-a-repository for more details.
> hint: `prek auto-update` often fixes this",
>
> https://github.com/sayanarijit/cottage
>   updating rev `main` -> `v0.5.2`
> prek installed at `.git/hooks/pre-commit`
> ```

With this setup:

When you run `git commit`, any modified secrets will be automatically encrypted:

> ```bash,test,session=myproject:36
> echo "DB_PASSWORD=updated-password" > secret2.env
> git commit --allow-empty -am "Update secrets"
> ```
>
> ```stdout
>   cottage-encrypt..........................................................Failed
> - hook id: cottage-sync-encrypt
> - files were modified by this hook
>
>   encrypt secret1.env
>      into secret1.env.cott.age
>      edit secret1.env.cott.toml
> ```

Let's try again

> ```bash,test,session=myproject:37
> git add .
> git commit -m "Update secrets"
> git push origin main
> ```
>
> ```stdout
> cottage-encrypt..........................................................Passed
> [main XXXXXXX] Update secrets
>  3 files changed, 9 insertions(+), 4 deletions(-)
> Enumerating objects: 9, done.
> Counting objects: 100% (9/9), done.
> Delta compression using up to 20 threads
> Compressing objects: 100% (5/5), done.
> Writing objects: 100% (5/5), X.XX KiB | X.XX MiB/s, done.
> Total 5 (delta 2), reused 0 (delta 0), pack-reused 0 (from 0)
> To /tmp/upstream.git
>    XXXXXXX..XXXXXXX  main -> main
> ```

Now let's pull the changes in the original repo:

TODO
