# Adding Recipient and Decrypting

Scenarios for adding recipient and decrypting secrets:

1.  [I want to decrypt secrets in the cloned repository](#i-want-to-decrypt-secrets-in-the-cloned-repository)
2.  [I got a checksum mismatch error when decrypting secrets](#i-got-a-checksum-mismatch-error-when-decrypting-secrets)

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
> ssh-keygen -t rsa -f .cottage/identity -N ""
> mv -v .cottage/identity.pub .cottage/recipients/newuser
> ```
>
> ```stdout
> Generating public/private rsa key pair.
> Your identification has been saved in .cottage/identity
> Your public key has been saved in .cottage/identity.pub
> The key fingerprint is:
> ...XXX...
> renamed '.cottage/identity.pub' -> '.cottage/recipients/newuser'
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
