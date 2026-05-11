# Initializing cottage

Scenarios for initializing cottage in a new or existing git repository:

1.  [I want to create a fresh new git repo and keep secrets in it](#i-want-to-create-a-fresh-new-git-repo-and-keep-secrets-in-it)
2.  [I want to add cottage to an existing git repo](#i-want-to-add-cottage-to-an-existing-git-repo)
3.  [I want to undo ctg init](#i-want-to-undo-ctg-init)

## I want to create a fresh new git repo and keep secrets in it

To start a fresh new repo with secrets, run:

> ```bash,test,newdir
> mkdir myproject
> cd myproject
> git init
>
> ctg init
> ```
>
> ```stdout
> Initialized empty Git repository in /tmp/tmp.XXX/.git/
> ```

To confirm that the repository is properly initialized, run:

> ```bash,test
> git status --short
> ```
>
> ```stdout
> ?? .cottage/
> ?? .gitattributes
> ?? .gitignore
> ```

Check the contents in the `.cottage` directory:

> ```bash,test
> tree .cottage
> ```
>
> ```stdout
> .cottage
> ├── identity
> └── recipients
>     └── XXX
>
> 2 directories, 2 files
> ```

Check the contents of `.gitignore` and `.gitattributes`:

> ```bash,test
> cat .gitignore
> ```
>
> ```stdout
> /.cottage/identity
> ```

> ```bash,test
> cat .gitattributes
> ```
>
> ```stdout
> *.cott.age binary export-ignore filter=cottage-encrypted -diff
> ```

## I want to add cottage to an existing git repo

To add cottage to an existing git repository (e.g. sayanarijit/jf), run:

> ```bash,test,newdir
> git clone git@github.com:sayanarijit/jf.git
> cd jf
>
> ctg init
> ```

To confirm that the repository is properly initialized, run:

> ```bash,test
> git status --short
> ```
>
> ```stdout
> M .gitignore
> ?? .cottage/
> ?? .gitattributes
> ```

> ```bash,test
> tree .cottage
> ```
>
> ```stdout
> .cottage
> ├── identity
> └── recipients
>     └── XXX
> ```

To confirm that `.gitignore` and `.gitattributes` are properly updated, run:

> ```bash,test
> grep .cottage/identity .gitignore
> ```
>
> ```stdout
> /.cottage/identity
> ```

> ```bash,test
> grep .cott.age .gitattributes
> ```
>
> ```stdout
> *.cott.age binary export-ignore filter=cottage-encrypted -diff
> ```

## I want to undo ctg init

For some reason, if you want to undo the `ctg init` command, you can run:

> ```bash,test
> ctg clean --all
>
> git status --short
> # no output
> ```
