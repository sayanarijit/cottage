# Scenarios: Configuring Secret Providers & Plugins

Cottage allows you to synchronize your secrets with external providers such as HashiCorp Vault, AWS Secrets Manager, or any custom API. This is useful for teams that want to use Git as the primary source of truth for configuration while still backing up or distributing secrets through a centralized enterprise vault.

Scenarios for configuring secret providers:

1. [I want to sync secrets with a non-git secret provider](#i-want-to-sync-secrets-with-a-non-git-secret-provider)
2. [I want to use a provider-specific plugin for cottage](#i-want-to-use-a-provider-specific-plugin-for-cottage)

## I want to sync secrets with a non-git secret provider

In this scenario, we configure a custom upstream provider using shell scripts to pull and push secrets from a mock API.

First, let's define the upstream in `cottage.toml` at the project root:

> ```bash,test,session=myproject:48
> cd /tmp/myproject
> cat > cottage.toml <<EOF
> [upstream.myvault]
> vars = { HOST = "vault.example.com" }
>
> [upstream.myvault.pull]
> script = 'echo "{\"API_KEY\": \"fetched-from-remote\"}"'
>
> [upstream.myvault.push]
> script = 'cat /dev/stdin > /dev/null && echo "Secret pushed successfully" 1>&2'
> EOF
> ```

Now, let's create a secret and link it to this upstream. Create a file `api-secret.json`:

> ```bash,test,session=myproject:49
> echo '{"API_KEY": "local-value"}' > api-secret.json
> ctg encrypt api-secret.json
> ```
>
> ```stdout
> encrypt api-secret.json
>    into api-secret.json.cott.age
>    edit .gitignore
>    edit api-secret.json.cott.toml
> ```

Edit `api-secret.json.cott.toml` to link it to the `myvault` upstream:

> ```bash,test,session=myproject:50
> cat >> api-secret.json.cott.toml <<EOF
>
> [upstream.myvault]
> pull = true
> push = true
> EOF
> ```

Now you can pull the secret from the remote provider. This will overwrite the local encrypted secret with the value from the provider:

> ```bash,test,session=myproject:51
> ctg pull myvault api-secret.json.cott.age
> ```
>
> ```stdout
> pull    myvault
>    into api-secret.json.cott.age
>    edit api-secret.json.cott.toml
> ```

Verify that the secret has been updated (you'll need to decrypt it first):

> ```bash,test,session=myproject:52
> ctg run cat api-secret.json.cott.age
> ```
>
> ```stdout
> {"API_KEY": "fetched-from-remote"}
> decrypt api-secret.json.cott.age
>    into api-secret.json
> delete api-secret.json
> ```

You can also push your local secret to the remote provider:

> ```bash,test,session=myproject:53
> ctg push myvault api-secret.json.cott.age
> ```
>
> ```stdout
> push    api-secret.json.cott.age
>    into myvault
> ```

## I want to use a provider-specific plugin for cottage

Cottage supports plugins, which are external binaries that handle the logic for interacting with specific secret providers.

A plugin is any binary that takes `pull` or `push` as the first argument, reads environment variables, and communicates via stdin/stdout.

To use a plugin, define it in `cottage.toml`:

> ```bash,test,session=myproject:54
> cat > cottage.toml <<EOF
> [upstream.vault]
> plugin = "./cottage-plugin-vault"
> vars = {
>   VAULT_ADDR = "https://vault.example.com"
> }
> EOF
> ```

For this scenario, let's mock a plugin named `cottage-plugin-vault` in our project directory:

> ```bash,test,session=myproject:55
> cat > /tmp/myproject/cottage-plugin-vault <<EOF
> #!/bin/sh
> action=\$1
> if [ "\$action" = "pull" ]; then
>   echo "{\"db_password\": \"plugin-secret\"}"
> elif [ "\$action" = "push" ]; then
>   cat /dev/stdin > /dev/null
>   echo "Pushed to vault" >&2
> fi
> EOF
> chmod +x /tmp/myproject/cottage-plugin-vault
> ```

Now, link a secret to the `vault` upstream in its metadata file `db-secret.json.cott.toml`:

> ```bash,test,session=myproject:56
> echo '{"db_password": "local-password"}' > db-secret.json
> ctg encrypt db-secret.json
> cat >> db-secret.json.cott.toml <<EOF
>
> [upstream.vault]
> pull = true
> push = true
> EOF
> ```

Pull the secret using the plugin:

> ```bash,test,session=myproject:57
> ctg pull vault db-secret.json.cott.age
> ```
>
> ```stdout
> pull    vault
>    into db-secret.json.cott.age
>    edit db-secret.json.cott.toml
> ```

Verify the update:

> ```bash,test,session=myproject:58
> ctg run cat db-secret.json.cott.age
> ```
>
> ```stdout
> {"db_password": "plugin-secret"}
> decrypt db-secret.json.cott.age
>    into db-secret.json
> delete db-secret.json
> ```

> [!TIP]
> See the configuration specification for more details on how to configure upstreams and plugins: [Configuration Specification](SPECIFICATION.md).
