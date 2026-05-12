# Diff, Status, and Sync

Scenarios for collaborating with others:

1.  [I want to compare locally edited secrets with upstream changes](#i-want-to-compare-locally-edited-secrets-with-upstream-changes)
2.  [I want to sync locally modified changes with upstream](#i-want-to-sync-locally-modified-changes-with-upstream)

## I want to compare locally edited secrets with upstream changes

If you have modified a secret locally and someone else has pushed changes to the same secret, you can compare them using `ctg diff`.

First, let's assume you have edited a secret locally:

> ```bash,test,session=myproject:26
> cd /tmp/myproject-clone
> echo "DB_PASSWORD=my-local-password" > secret1.env
> ```

Now, you pull the latest changes from the upstream repository:

> ```bash,test,session=myproject:27
> git pull origin main
> ```

> [!NOTE]
> Since `secret1.env` is ignored by git, there will be no git conflict.
> However, `secret1.env.cott.age` (the encrypted file) and `secret1.env.cott.toml` will be updated.

Now you can compare your local changes with the upstream version:

> ```bash,test,session=myproject:28
> ctg diff
> ```
>
> ```diff,stdout
> diff --git a/secret1.env b/secret1.env
> --- a/secret1.env
> +++ b/secret1.env
> @@ -1 +1 @@
> -DB_PASSWORD=my-local-password
> +DB_PASSWORD=edited-by-admin
> ```

## I want to sync locally modified changes with upstream

If you want to update the encrypted files with your local changes, you can use `ctg sync`.

First, check the status of your secrets:

> ```bash,test,session=myproject:29
> ctg status
> ```
>
> ```stdout
> encrypt secret1.env
>    into secret1.env.cott.age
> ```

Now run `ctg sync` to encrypt the modified files:

> ```bash,test,session=myproject:30
> ctg sync
> ```
>
> ```stdout
> encrypt secret1.env
>    into secret1.env.cott.age
>    edit secret1.env.cott.toml
> ```

Verify that everything is in sync:

> ```bash,test,session=myproject:31
> ctg status
> # No output means everything is in sync
> ```
