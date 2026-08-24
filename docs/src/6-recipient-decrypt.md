# Adding Recipient and Decrypting

Scenarios for adding recipient and decrypting secrets:

1.  [I want to decrypt secrets in the cloned repository](#i-want-to-decrypt-secrets-in-the-cloned-repository)
2.  [I got a checksum mismatch error when decrypting secrets](#i-got-a-checksum-mismatch-error-when-decrypting-secrets)
3.  [I want to run a command with decrypted secrets (ctg run / ctgx)](#i-want-to-run-a-command-with-decrypted-secrets-ctg-run--ctgx)
4.  [I want to ensure decrypted secrets are cleaned up after running a command (ctg run --clean)](#i-want-to-ensure-decrypted-secrets-are-cleaned-up-after-running-a-command-ctg-run---clean)

## I want to decrypt secrets in the cloned repository

Let's try to decrypt secrets in the cloned repository:

> ```bash,test,error=1,session=myproject:20
> cd /tmp/myproject-clone
> ctg decrypt
> ```
>
> ```stderr
> Error: No matching keys found
> ```

Right... You need to set up your keys first. Let's add keys first.

> ```bash,test,session=myproject:21
> ctg keygen -n newuser
>
> tree .cottage
> ```
>
> ```stdout
> .cottage
> ├── identity
> └── recipients
>     ├── newuser
>     └── sayanarijit
> ```

Let's commit and push the changes to the remote repository, so that someone with access (admin) can pull the changes and re-encrypt the secrets for the new key:

> ```bash,test,session=myproject:22
> git add .cottage/recipients/newuser
> git commit -m "Add new recipient key"
> git push origin main
> ```
>
> ```stdout
> [main XXXXXXX] Add new recipient key
>  1 file changed, 1 insertion(+)
>  create mode 100644 .cottage/recipients/newuser
> Enumerating objects: 8, done.
> Counting objects: 100% (8/8), done.
> Delta compression using up to 20 threads
> Compressing objects: 100% (4/4), done.
> Writing objects: 100% (5/5), X.XX KiB | X.XX MiB/s, done.
> Total 5 (delta 1), reused 0 (delta 0), pack-reused 0 (from 0)
> To /tmp/upstream.git
>    XXXXXXX..XXXXXXX  main -> main
> ```

Now admin should pull the changes and re-encrypt the secrets for the new key.

> ```bash,test,session=myproject:23
> cd /tmp/myproject
> git pull origin main
> ```
>
> ```diff,stdout
> remote: Enumerating objects: 8, done.
> remote: Counting objects: 100% (8/8), done.
> remote: Compressing objects: 100% (4/4), done.
> remote: Total 5 (delta 1), reused 0 (delta 0), pack-reused 0 (from 0)
> Unpacking objects: 100% (5/5), X.XX KiB | X.XX MiB/s, done.
> From /tmp/upstream
>  * branch            main       -> FETCH_HEAD
>    XXXXXXX..XXXXXXX  main       -> origin/main
> Updating XXXXXXX..XXXXXXX
> Fast-forward
>  .cottage/recipients/newuser | 1 +
>  1 file changed, 1 insertion(+)
>  create mode 100644 .cottage/recipients/newuser
> ```

> ```bash,test,session=myproject:24
> ctg decrypt --force && ctg encrypt
> ```
>
> ```stdout
> decrypt secret1.env.cott.age
>    into secret1.env
> decrypt secret2.env.cott.age
>    into secret2.env
> encrypt secret1.env
>    into secret1.env.cott.age
>    edit secret1.env.cott.toml
> encrypt secret2.env
>    into secret2.env.cott.age
>    edit secret2.env.cott.toml
> ```

> [!NOTE]
> The `--force` flag is used to bypass the checksum verification when decrypting secrets.
> This is necessary when adding a new recipient key, because the encrypted secret files and recipient checksum in the TOML files need to be updated.

> ```bash,test,session=myproject:25
> git diff
> ```
>
> ```diff,stdout
> diff --git a/secret1.env.cott.age b/secret1.env.cott.age
> index XXXXXXX..XXXXXXX 100644
> Binary files a/secret1.env.cott.age and b/secret1.env.cott.age differ
> diff --git a/secret1.env.cott.toml b/secret1.env.cott.toml
> index XXXXXXX..XXXXXXX 100644
> --- a/secret1.env.cott.toml
> +++ b/secret1.env.cott.toml
> @@ -1,6 +1,6 @@
>  [checksum]
> -encrypted = "blake3:...XXX..."
> -recipients = "blake3:...XXX..."
> +encrypted = "blake3:...XXX..."
> +recipients = "blake3:...XXX..."
>
>  [preview]
>  format = "dotenv"
> diff --git a/secret2.env.cott.age b/secret2.env.cott.age
> index XXXXXXX..XXXXXXX 100644
> Binary files a/secret2.env.cott.age and b/secret2.env.cott.age differ
> diff --git a/secret2.env.cott.toml b/secret2.env.cott.toml
> index XXXXXXX..XXXXXXX 100644
> --- a/secret2.env.cott.toml
> +++ b/secret2.env.cott.toml
> @@ -1,6 +1,6 @@
>  [checksum]
> -encrypted = "blake3:...XXX..."
> -recipients = "blake3:...XXX..."
> +encrypted = "blake3:...XXX..."
> +recipients = "blake3:...XXX..."
>
>  [preview]
>  format = "dotenv"
> ```

Admin will commit and push the re-encrypted secrets to the remote repository:

> ```bash,test,session=myproject:24
> git add .
> git commit -m "Re-encrypt secrets for new recipient key"
> git push origin main
> ```
>
> ```stdout
> [main XXXXXXX] Re-encrypt secrets for new recipient key
>  4 files changed, 4 insertions(+), 4 deletions(-)
> ```

Now you can pull the changes in the cloned repository and decrypt the secrets:

> ```bash,test,session=myproject:25
> cd /tmp/myproject-clone
> git pull origin main
>
> ctg decrypt
> ```
>
> ```stdout
> remote: Enumerating objects: 9, done.
> remote: Counting objects: 100% (9/9), done.
> remote: Compressing objects: 100% (6/6), done.
> remote: Total 6 (delta 1), reused 0 (delta 0), pack-reused 0 (from 0)
> Unpacking objects: 100% (6/6), X.XX KiB | XXX.XX KiB/s, done.
> From /tmp/upstream
>  * branch            main       -> FETCH_HEAD
>    XXXXXXX..XXXXXXX  main       -> origin/main
> Updating XXXXXXX..XXXXXXX
> Fast-forward
>  secret1.env.cott.age  | Bin XXX -> XXX bytes
>  secret1.env.cott.toml |   4 ++--
>  secret2.env.cott.age  | Bin XXX -> XXX bytes
>  secret2.env.cott.toml |   4 ++--
>  4 files changed, 4 insertions(+), 4 deletions(-)
> decrypt secret1.env.cott.age
>    into secret1.env
> decrypt secret2.env.cott.age
>    into secret2.env
> ```

## I got a checksum mismatch error when decrypting secrets

> [!WARNING]
> Checksum mismatch error indicates that the encrypted secret file or recipient has been tampered with or corrupted.
> Please verify the integrity of the encrypted secret with the admin.

If you are sure that the encrypted secret file and recipient are correct, you can bypass the checksum verification by running:

> ```bash
> ctg decrypt --force
> ```

## I want to run a command with decrypted secrets (ctg run / ctgx)

To run a command (such as scripts, deployment tools, or applications) that needs access to secrets on disk, use `ctg run` or its shortcut `ctgx`.

`ctg run` decrypts the target encrypted files before executing the command, and automatically manages the lifecycle of the decrypted files:

- **If the decrypted secret was not present on disk beforehand**: `ctg run` decrypts it temporarily for the command, and automatically deletes (cleans up) the decrypted file after the command completes (regardless of success or failure).
- **If the decrypted secret was already present on disk beforehand**: `ctg run` leaves it on disk after the command completes.

> ```bash
> # Decrypts secret1.env.cott.age temporarily, runs the command, and deletes secret1.env afterwards
> ctg run cat secret1.env.cott.age
>
> # Shortcut syntax with ctgx
> ctgx ./deploy.sh
> ```

## I want to ensure decrypted secrets are cleaned up after running a command (ctg run --clean)

If you want to guarantee that all decrypted secrets are deleted after running a command—even if they were already present on disk before running the command—pass the `--clean` flag:

> ```bash
> ctg run --clean ./deploy.sh
>
> # Or using ctgx
> ctgx --clean ./deploy.sh
> ```

This decrypts missing secrets before running the command, executes the command, and ensures all target decrypted secrets are deleted upon completion.
