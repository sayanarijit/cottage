# Encrypting Secrets

Scenarios for encrypting new or existing secrets:

1.  [I want to create a new encrypted file](#i-want-to-create-a-new-encrypted-file)
2.  [I want to encrypt an existing cleartext file](#i-want-to-encrypt-an-existing-cleartext-file)
3.  [I want the cleartext secret deleted after encryption](#i-want-the-cleartext-secret-deleted-after-encryption)

## I want to create a new encrypted file

There are many ways to create a new encrypted file. The simplest way is to use the `ctg encrypt` command:

> ```bash,test
> cat > secret1.env <<EOF
> DB_PASSWORD=supersecret
> EOF
>
> ctg encrypt secret1.env
> ```
>
> ```stdout
> encrypt secret1.env
>    into secret1.env.cott.age
>    edit .gitignore
>    edit secret1.env.cott.toml
> ```

> ```bash,test
> ctg edit secret2.env  # This will open the file in $EDITOR
>
> # But you can also provide the content using stdin
> ctg edit secret2.env <<EOF
> DB_PASSWORD=supersecret
> EOF
> ```
>
> ```stdout
> edit secret2.env
>    into secret2.env.cott.age
>    edit .gitignore
>    edit secret2.env.cott.toml
> ```

Let's verify what it did:

> ```bash,test
> ls -1
> ```
>
> ```stdout
> secret1.env
> secret1.env.cott.age
> secret1.env.cott.toml
> secret2.env
> secret2.env.cott.age
> secret2.env.cott.toml
> ```

> ```bash,test
> cat .gitignore
> ```
>
> ```stdout
> /.cottage/identity
> /secret1.env
> /secret2.env
> ```

> ```bash,test
> cat secret1.env.cott.toml
> ```
>
> ```toml,stdout
> [checksum]
> encrypted = "blake3:65d42dc970a4d6f6726df1aa19692b365af1a882c4cc1893a7b2ff9f9ef89bcf"
> recipients = "blake3:2821590e2c915e409228660ff185130496579a221f5810d296d35ce93b26c8f3"
>
> [preview]
> format = "dotenv"
> preview = """
> DB_PASSWORD=2026-05-11T12:44:02.427264873+00:00
> """
>
> [secret]
> timestamp = "2026-05-11T12:44:02.427264873+00:00"
> ```

> ```bash,test
> cat secret1.env.cott.age
> ```
>
> ```stdout
> age-encryption.org/v1
> XXX
> ```

## I want to encrypt an existing cleartext file

Same as above.

## I want the cleartext secret deleted after encryption

Just add `--clean` flag to the `ctg encrypt` or `ctg edit` command:

> ```bash,test
> ctg edit --clean secret1.env <<EOF
> DB_PASSWORD=editedsecret
> EOF
>
> ctg encrypt secret3.env --clean
> ```
>
> ```stdout
> encrypt secret1.env
>    into secret1.env.cott.age
>    edit secret1.env.cott.toml
> delete  secret1.env
> ```

If there is no change, re-encryption will be skipped, but the cleartext file will still be deleted:

> ```bash,test
> ctg encrypt --clean secret2.env
> ```
>
> ```
> delete  secret2.env
> ```

But the entries in `.gitignore` will still remain:

> ```bash,test
> cat .gitignore
> ```
>
> ```stdout
> /.cottage/identity
> /secret1.env
> /secret2.env
> ```
