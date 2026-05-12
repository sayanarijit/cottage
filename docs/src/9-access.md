# Access Control

You can control which recipients are allowed or denied from decrypting specific secrets using the `allow` and `deny` fields in the `[secret]` section of the corresponding `*.cott.toml` metadata file.

Scenarios for managing access control:

1. [I want to allow only prod-server to be able to decrypt prod-secret](#i-want-to-allow-only-prod-server-to-be-able-to-decrypt-prod-secret)
2. [I want to allow everyone except prod-server to be able to decrypt dev-secret](#i-want-to-allow-everyone-except-prod-server-to-be-able-to-decrypt-dev-secret)
3. [I want to ensure that access control rules are enforced for all secrets before I merge a pull request](#i-want-to-ensure-that-access-control-rules-are-enforced-for-all-secrets-before-i-merge-a-pull-request)

## I want to allow only prod-server to be able to decrypt prod-secret

In this scenario, we restrict access to a production secret so that only the production server can decrypt it.

First, let's add a new recipient key for the production server in the admin workspace:

> ```bash,test,session=myproject:37
> cd /tmp/myproject
> ssh-keygen -t ed25519 -f .cottage/prod-server.key -N ""
> cp .cottage/prod-server.key.pub .cottage/recipients/prod-server
> ```

Now, create and encrypt the production secret:

> ```bash,test,session=myproject:38
> echo "prod-db-password: supersecret" > prod-secret.yml
> ctg encrypt prod-secret.yml
> ```
>
> ```stdout
> encrypt prod-secret.yml
>    into prod-secret.yml.cott.age
>    edit .gitignore
>    edit prod-secret.yml.cott.toml
> ```

Edit the generated `prod-secret.yml.cott.toml` file to add the `allow` rule:

> ```bash,test,session=myproject:39
> sed -i '/\[secret\]/a allow = ["prod-server"]' prod-secret.yml.cott.toml
> cat prod-secret.yml.cott.toml
> ```
>
> ```toml,stdout
> [checksum]
> encrypted = "blake3:...XXX..."
> recipients = "blake3:...XXX..."
>
> [preview]
> format = "yaml"
> preview = """
> prod-db-password: "...XXX..."
> """
>
> [secret]
> allow = ["prod-server"]
> timestamp = "...XXX..."
> ```

Re-encrypt the secret to apply the new access rules:

> ```bash,test,session=myproject:40
> ctg encrypt prod-secret.yml
> ```
>
> ```stdout
> encrypt prod-secret.yml
>    into prod-secret.yml.cott.age
>    edit prod-secret.yml.cott.toml
> ```

Now, only someone with the `prod-server` key can decrypt this secret. Even the `admin` who created it will be denied access unless they are explicitly added to the `allow` list.

Verify that access is denied for the default identity (`admin`):

> ```bash,test,error=1,session=myproject:41
> rm prod-secret.yml
> ctg decrypt prod-secret.yml.cott.age
> ```
>
> ```stderr
> Error: No matching keys found
> ```

Verify that access is granted for the `prod-server` key:

> ```bash,test,session=myproject:42
> ctg decrypt prod-secret.yml.cott.age -i .cottage/prod-server.key
> cat prod-secret.yml
> ```
>
> ```stdout
> decrypt prod-secret.yml.cott.age
>    into prod-secret.yml
> prod-db-password: supersecret
> ```

## I want to allow everyone except prod-server to be able to decrypt dev-secret

In this scenario, we allow all team members and admins to access a development secret, but prevent the production server from being able to decrypt it.

Create and encrypt the development secret:

> ```bash,test,session=myproject:43
> echo "dev-db-password: devsecret" > dev-secret.yml
> ctg encrypt dev-secret.yml
> ```
>
> ```stdout
> encrypt dev-secret.yml
>    into dev-secret.yml.cott.age
>    edit dev-secret.yml.cott.toml
> ```

Edit the `dev-secret.yml.cott.toml` file to add the `deny` rule:

> ```bash,test,session=myproject:44
> sed -i '/\[secret\]/a deny = ["prod-server"]' dev-secret.yml.cott.toml
> cat dev-secret.yml.cott.toml
> ```
>
> ```toml,stdout
> [checksum]
> encrypted = "blake3:...XXX..."
> recipients = "blake3:...XXX..."
>
> [preview]
> format = "yaml"
> preview = """
> dev-db-password: "...XXX..."
> """
>
> [secret]
> deny = ["prod-server"]
> timestamp = "...XXX..."
> ```

Re-encrypt to apply the rules:

> ```bash,test,session=myproject:45
> ctg encrypt dev-secret.yml
> ```
>
> ```stdout
> encrypt dev-secret.yml
>    into dev-secret.yml.cott.age
>    edit dev-secret.yml.cott.toml
> ```

Verify that the `admin` can still decrypt the secret (since they are not `prod-server` and are in the recipients list):

> ```bash,test,session=myproject:46
> rm dev-secret.yml
>
> ctg decrypt dev-secret.yml.cott.age
>
> cat dev-secret.yml
> ```
>
> ```stdout
> decrypt dev-secret.yml.cott.age
>    into dev-secret.yml
> dev-db-password: devsecret
> ```

Verify that the `prod-server` is denied access:

> ```bash,test,error=1,session=myproject:47
> rm dev-secret.yml
> COTTAGE_IDENTITY=.cottage/prod-server.key ctg decrypt dev-secret.yml.cott.age
> ```
>
> ```stderr
> Error: No matching keys found
> ```

> [!TIP]
> You can use glob patterns in `allow` and `deny` rules. For example, `allow = ["team-*"]` would allow any recipient whose name starts with `team-`.

## I want to ensure that access control rules are enforced for all secrets before I merge a pull request

To ensure that access control rules are properly set up for all secrets before merging a pull request, you can add run `ctg verify` in a CI workflow or a git hook.

Example using GitHub Actions:

```yaml
# .github/workflows/cottage-verify.yml
name: Cottage Verify
on: [push, pull_request]
permissions:
  contents: read
jobs:
  verify-secrets:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Verify secrets
        run: docker run --rm -v "${{ github.workspace }}:/app" ghcr.io/sayanarijit/cottage verify
```

This way, if someone pushes a secret with incorrect access control rules, or without updating the checksum in metadata, the verification will fail and prevent the pull request from being merged until the issues are resolved.
