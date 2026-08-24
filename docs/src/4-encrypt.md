# Encrypting Secrets

Scenarios for encrypting and editing new or existing secrets:

1.  [I want to create a new encrypted file](#i-want-to-create-a-new-encrypted-file)
2.  [I want to encrypt an existing cleartext file](#i-want-to-encrypt-an-existing-cleartext-file)
3.  [I want to re-encrypt all secrets in the current directory](#i-want-to-re-encrypt-all-secrets-in-the-current-directory)
4.  [I want to edit an existing secret with automatic cleanup or retention](#i-want-to-edit-an-existing-secret-with-automatic-cleanup-or-retention)
5.  [I want the cleartext secret deleted after encryption or editing](#i-want-the-cleartext-secret-deleted-after-encryption-or-editing)

## I want to create a new encrypted file

There are many ways to create a new encrypted file. The simplest way is to use the `ctg encrypt` command:

> ```bash,test,session=myproject:7
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

> ```bash,test,session=myproject:8
> # ctg edit secret2.env  # This will open the file in $EDITOR
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

> ```bash,test,session=myproject:9
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

> ```bash,test,session=myproject:10
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
> encrypted = "blake3:...XXX..."
> recipients = "blake3:...XXX..."
>
> [preview]
> format = "dotenv"
> preview = """
> DB_PASSWORD=XXXX-XX-XXTXX:XX:XX.XXXXXXXXX+00:00
> """
>
> [secret]
> timestamp = "XXXX-XX-XXTXX:XX:XX.XXXXXXXXX+00:00"
> ```

> ```bash,test
> cat secret1.env.cott.age
> ```
>
> ```stdout
> age-encryption.org/v1
> ...XXX...
> ```

## I want to encrypt an existing cleartext file

Same as above.

## I want to re-encrypt all secrets in the current directory

Just run `ctg encrypt` without any file argument to encrypt files that require encryption:

> ```bash,test,session=myproject:11
> ctg encrypt
> # There is no change, so the encryption will be skipped
> ```

To force re-encryption, add `--force` flag:

> ```bash,test,session=myproject:12
> ctg encrypt --force
> ```
>
> ```stdout
> encrypt secret1.env
>    into secret1.env.cott.age
>    edit secret1.env.cott.toml
> encrypt secret2.env
>    into secret2.env.cott.age
>    edit secret2.env.cott.toml
> ```

## I want to edit an existing secret with automatic cleanup or retention

`ctg edit` decrypts the secret before opening it in `$EDITOR` (or reading stdin) and re-encrypts upon save:

- **If the cleartext file was already present on disk**: `ctg edit` keeps the cleartext file after saving and re-encrypting.
- **If the cleartext file was not present on disk** (only `.cott.age` was tracked): `ctg edit` decrypts it temporarily for editing, re-encrypts, and then automatically cleans up (deletes) the cleartext file.

## I want the cleartext secret deleted after encryption or editing

Just add `--clean` flag to the `ctg encrypt` or `ctg edit` command. The `--clean` flag ensures that the decrypted cleartext file is deleted even if it was present on disk before running the command:

> ```bash,test,session=myproject:13
> ctg edit --clean secret1.env <<EOF
> DB_PASSWORD=editedsecret
> EOF
> ```
>
> ```stdout
> encrypt secret1.env
>    into secret1.env.cott.age
>    edit secret1.env.cott.toml
> delete  secret1.env
> ```

If there is no change, re-encryption will be skipped, but the cleartext file will still be deleted:

> ```bash,test,session=myproject:14
> ctg encrypt --clean secret2.env
> ```
>
> ```
> delete  secret2.env
> ```

But the entries in `.gitignore` will still remain:

> ```bash,test,session=myproject:15
> cat .gitignore
> ```
>
> ```stdout
> /.cottage/identity
> /secret1.env
> /secret2.env
> ```
