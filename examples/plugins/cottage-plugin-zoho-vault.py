#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "pyreqwest>=0.10.1",
# ]
# ///


# cottage.toml
"""
[upstream.dev-zoho]
envfile = "./zoho/dev.env.cott.age"  # Should export ZOHO_CLIENT_ID, ZOHO_CLIENT_SECRET, ZOHO_REFRESH_TOKEN.
vars = {
  ZOHO_SECRET_ID = "1234567890",
  ZOHO_VAULT_URL = "https://vault.zoho.com",
}
plugin = "./examples/plugins/cottage-plugin-zoho-vault.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-zoho]
pull = true
push = true
"""

import json
import os
import sys

from cyclopts import App
from pydantic import BaseModel, Field
from pyreqwest.client import SyncClientBuilder


class ZohoVaultConfig(BaseModel):
    model_config = {"extra": "ignore"}
    zoho_client_id: str = Field(..., alias="ZOHO_CLIENT_ID")
    zoho_client_secret: str = Field(..., alias="ZOHO_CLIENT_SECRET")
    zoho_refresh_token: str = Field(..., alias="ZOHO_REFRESH_TOKEN")
    zoho_secret_id: str = Field(..., alias="ZOHO_SECRET_ID")
    zoho_vault_url: str = Field("https://vault.zoho.com", alias="ZOHO_VAULT_URL")
    zoho_passphrase: str | None = Field(None, alias="ZOHO_PASSPHRASE")

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


def get_access_token(cfg: ZohoVaultConfig) -> str:
    # Resolve regional accounts URL
    accounts_url = "https://accounts.zoho.com/oauth/v2/token"
    if ".zoho.eu" in cfg.zoho_vault_url:
        accounts_url = "https://accounts.zoho.eu/oauth/v2/token"
    elif ".zoho.com.cn" in cfg.zoho_vault_url:
        accounts_url = "https://accounts.zoho.com.cn/oauth/v2/token"
    elif ".zoho.in" in cfg.zoho_vault_url:
        accounts_url = "https://accounts.zoho.in/oauth/v2/token"
    elif ".zoho.com.au" in cfg.zoho_vault_url:
        accounts_url = "https://accounts.zoho.com.au/oauth/v2/token"

    print(f"Refreshing OAuth2 token from {accounts_url}...", file=sys.stderr)
    with SyncClientBuilder().build() as client:
        resp = (
            client.post(accounts_url)
            .query(
                {
                    "refresh_token": cfg.zoho_refresh_token,
                    "client_id": cfg.zoho_client_id,
                    "client_secret": cfg.zoho_client_secret,
                    "grant_type": "refresh_token",
                }
            )
            .build()
            .send()
        )
        data = resp.json()
        if "access_token" not in data:
            print(f"OAuth refresh failed: {data}", file=sys.stderr)
            sys.exit(1)
        return data["access_token"]


def decrypt_data(encrypted_payload: str, passphrase: str | None) -> dict:
    """
    Decryption Hook for Zoho Vault Host-Proof Security.

    Zoho Vault returns secrets encrypted on the server. To decrypt, you must use the
    PBKDF2/AES-256 decryption utility provided in the Zoho Vault API documentation/files.
    Contact support@zohovault.com to obtain their official crypto helper files (e.g. `zohovault_crypto.py`).
    """
    if not passphrase:
        print(
            "Warning: ZOHO_PASSPHRASE not set. Returning encrypted payload directly.",
            file=sys.stderr,
        )
        return {"encrypted_data": encrypted_payload}

    # Standard Import placeholder:
    # try:
    #     import zohovault_crypto
    #     return zohovault_crypto.decrypt(encrypted_payload, passphrase)
    # except ImportError:
    #     pass

    # For demonstration/custom implementations:
    print(
        "Please integrate the zohovault_crypto module to perform host-proof decryption.",
        file=sys.stderr,
    )
    return {"encrypted_data": encrypted_payload}


def encrypt_data(plain_payload: dict, passphrase: str | None) -> str:
    """
    Encryption Hook for Zoho Vault Host-Proof Security.

    Encrypts the JSON payload on the client side before uploading to Zoho Vault.
    """
    if not passphrase:
        print(
            "Warning: ZOHO_PASSPHRASE not set. Uploading plain string as payload.",
            file=sys.stderr,
        )
        return json.dumps(plain_payload)

    # Standard Import placeholder:
    # try:
    #     import zohovault_crypto
    #     return zohovault_crypto.encrypt(plain_payload, passphrase)
    # except ImportError:
    #     pass

    print(
        "Please integrate the zohovault_crypto module to perform host-proof encryption.",
        file=sys.stderr,
    )
    return json.dumps(plain_payload)


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = ZohoVaultConfig.model_validate(os.environ)
    token = get_access_token(cfg)

    urlpath = f"/api/rest/json/v1/secrets/{cfg.zoho_secret_id}"
    print(
        f"Pulling secret '{cfg.zoho_secret_id}' from Zoho Vault...",
        file=sys.stderr,
    )
    with (
        SyncClientBuilder()
        .base_url(cfg.zoho_vault_url)
        .default_headers(
            {
                "Authorization": f"Zoho-oauthtoken {token}",
                "Accept": "application/json",
            }
        )
        .error_for_status()
        .build()
    ) as client:
        resp = client.get(urlpath).build().send()
        response_json = resp.json()

    # Zoho Vault responses nest secret information inside a standard structure
    # e.g., {"operationName": "getSecret", "status": "Success", "secretData": "..."}
    secret_data_str = response_json.get("secretData", "")
    decrypted = decrypt_data(secret_data_str, cfg.zoho_passphrase)
    print(json.dumps(decrypted))


@app.command(name="push")
def cmd_push():
    cfg = ZohoVaultConfig.model_validate(os.environ)
    token = get_access_token(cfg)

    payload = json.loads(input())
    encrypted_payload_str = encrypt_data(payload, cfg.zoho_passphrase)

    urlpath = f"/api/rest/json/v1/secrets/{cfg.zoho_secret_id}"
    print(
        f"Pushing secret '{cfg.zoho_secret_id}' to Zoho Vault...",
        file=sys.stderr,
    )

    # Prepare form data required by Zoho Vault API
    form_data = {
        "INPUT_DATA": json.dumps(
            {
                "secretData": encrypted_payload_str,
                # Additional metadata fields can go here
            }
        )
    }

    with (
        SyncClientBuilder()
        .base_url(cfg.zoho_vault_url)
        .default_headers(
            {
                "Authorization": f"Zoho-oauthtoken {token}",
                "Accept": "application/json",
            }
        )
        .error_for_status()
        .build()
    ) as client:
        client.put(urlpath).form(form_data).build().send()


if __name__ == "__main__":
    app()
