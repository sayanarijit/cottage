# Syncing with Git

Scenarios for syncing secrets with git:

1.  [I want to push the encrypted secrets to git](#i-want-to-push-the-encrypted-secrets-to-git)
2.  [I want to pull the encrypted secrets from git](#i-want-to-pull-the-encrypted-secrets-from-git)

## I want to push the encrypted secrets to git

First, let's check the uncommitted changes we have in our local repo:

> ```bash,test,session=myproject:16
> git status --short
> ```
>
> ```stdout
> ?? .cottage/
> ?? .gitattributes
> ?? .gitignore
> ?? secret1.env.cott.age
> ?? secret1.env.cott.toml
> ?? secret2.env.cott.age
> ?? secret2.env.cott.toml
> ```

Let's create a new bare git repo and call it upstream:

> ```bash,test,session=myproject:17
> mkdir -p /tmp/upstream.git
> (cd /tmp/upstream.git && git init --bare)
> ```
>
> ```stdout
> Initialized empty Git repository in /tmp/upstream.git/
> ```

Now let's add the upstream to our local repo and push the encrypted secrets:

> ```bash,test,session=myproject:18
> git remote add origin /tmp/upstream.git
> git add .
> git commit -m "Add encrypted secrets"
> git push origin main
> ```
>
> ```stdout
> [main (root-commit) XXXXXXX] Add encrypted secrets
>  7 files changed, 29 insertions(+)
>  create mode 100644 .cottage/recipients/...XXX...
>  create mode 100644 .gitattributes
>  create mode 100644 .gitignore
>  create mode 100644 secret1.env.cott.age
>  create mode 100644 secret1.env.cott.toml
>  create mode 100644 secret2.env.cott.age
>  create mode 100644 secret2.env.cott.toml
> Enumerating objects: 11, done.
> Counting objects: 100% (11/11), done.
> Delta compression using up to 20 threads
> Compressing objects: 100% (8/8), done.
> Writing objects: 100% (11/11), X.XX KiB | X.XX MiB/s, done.
> Total 11 (delta 1), reused 0 (delta 0), pack-reused 0 (from 0)
> To /tmp/upstream.git
>  * [new branch]      main -> main
> ```

## I want to pull the encrypted secrets from git

Let's clone the upstream repo to a new directory and check the contents:

> ```bash,test,session=myproject:19
> cd /tmp
> git clone /tmp/upstream.git myproject-clone
> cd myproject-clone
> ls -A
> ```
>
> ```stdout
> Cloning into 'myproject-clone'...
> done.
> .cottage  .git  .gitattributes  .gitignore  secret1.env.cott.age  secret1.env.cott.toml  secret2.env.cott.age  secret2.env.cott.toml
> ```
