#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "azure-keyvault-secrets>=4.9.0",
#     "azure-identity>=1.16.1",
# ]
# ///


# cottage.toml
"""
[upstream.dev-azure]
envfile = "./azure/dev.env.cott.age"  # Should export AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, AZURE_TENANT_ID.
vars = {
  AZURE_KEYVAULT_URL = "https://my-keyvault.vault.azure.net/",
  AZURE_SECRET_NAME = "myapp-dev-secrets",
}
plugin = "./examples/plugins/cottage-plugin-azure-keyvault.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-azure]
pull = true
push = true
"""

import json
import os
import sys

from azure.identity import DefaultAzureCredential
from azure.keyvault.secrets import SecretClient
from cyclopts import App
from pydantic import BaseModel, Field


class AzureKeyVaultConfig(BaseModel):
    model_config = {"extra": "ignore"}
    azure_keyvault_url: str = Field(..., alias="AZURE_KEYVAULT_URL")
    azure_secret_name: str = Field(..., alias="AZURE_SECRET_NAME")

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = AzureKeyVaultConfig.model_validate(os.environ)
    credential = DefaultAzureCredential()
    client = SecretClient(vault_url=cfg.azure_keyvault_url, credential=credential)
    print(  # Use --debug to see this message
        f"Pulling secret '{cfg.azure_secret_name}' from Azure Key Vault '{cfg.azure_keyvault_url}'...",
        file=sys.stderr,
    )
    try:
        secret = client.get_secret(cfg.azure_secret_name)
    except Exception as e:
        print(f"Error retrieving secret from Azure Key Vault: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        # Try to return as JSON if it's a JSON object
        data = json.loads(secret.value)
        print(json.dumps(data))
    except json.JSONDecodeError:
        # Fallback to returning the raw string wrapped in a dict
        print(json.dumps({"value": secret.value}))


@app.command(name="push")
def cmd_push():
    cfg = AzureKeyVaultConfig.model_validate(os.environ)
    credential = DefaultAzureCredential()
    client = SecretClient(vault_url=cfg.azure_keyvault_url, credential=credential)
    payload_str = json.dumps(json.loads(input()))
    print(  # Use --debug to see this message
        f"Pushing secret '{cfg.azure_secret_name}' to Azure Key Vault '{cfg.azure_keyvault_url}'...",
        file=sys.stderr,
    )
    try:
        client.set_secret(cfg.azure_secret_name, payload_str)
    except Exception as e:
        print(f"Error setting secret in Azure Key Vault: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    app()
