#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "bitwarden-sdk>=0.3.1",
# ]
# ///


# cottage.toml
"""
[upstream.dev-bitwarden]
envfile = "./bitwarden/dev.env.cott.age"  # Should export BWS_ACCESS_TOKEN.
vars = {
  BW_SECRET_ID = "your-secret-uuid-here",
}
plugin = "./examples/plugins/cottage-plugin-bitwarden.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-bitwarden]
pull = true
push = true
"""

import json
import os
import sys

from bitwarden_sdk import BitwardenClient
from bitwarden_sdk.schemas import SecretUpdateRequest
from cyclopts import App
from pydantic import BaseModel, Field


class BWSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    bws_access_token: str = Field(..., alias="BWS_ACCESS_TOKEN")
    bw_secret_id: str = Field(..., alias="BW_SECRET_ID")
    bw_organization_id: str | None = Field(None, alias="BW_ORGANIZATION_ID")
    bw_project_id: str | None = Field(None, alias="BW_PROJECT_ID")

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


def get_bw_client(config: BWSecretConfig) -> BitwardenClient:
    client = BitwardenClient()
    client.access_token_login(config.bws_access_token)
    return client


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = BWSecretConfig.model_validate(os.environ)
    client = get_bw_client(cfg)
    print(  # Use --debug to see this message
        f"Pulling secret '{cfg.bw_secret_id}' from Bitwarden Secrets Manager...",
        file=sys.stderr,
    )
    try:
        secret = client.secrets().get(cfg.bw_secret_id)
    except Exception as e:
        print(f"Error retrieving secret from Bitwarden: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        data = json.loads(secret.value)
        print(json.dumps(data))
    except json.JSONDecodeError:
        print(json.dumps({"value": secret.value}))


@app.command(name="push")
def cmd_push():
    cfg = BWSecretConfig.model_validate(os.environ)
    client = get_bw_client(cfg)
    payload_str = json.dumps(json.loads(input()))
    print(  # Use --debug to see this message
        f"Pushing secret to Bitwarden Secrets Manager for ID '{cfg.bw_secret_id}'...",
        file=sys.stderr,
    )

    # Try updating the secret first
    try:
        secret = client.secrets().get(cfg.bw_secret_id)
        update_request = SecretUpdateRequest(
            key=secret.key,
            value=payload_str,
            note=secret.note or "Managed by Cottage",
            project_id=cfg.bw_project_id or getattr(secret, "project_id", None),
        )
        client.secrets().update(cfg.bw_secret_id, update_request)
        print(f"Updated Bitwarden secret '{secret.key}'", file=sys.stderr)
    except Exception as e:
        # If it doesn't exist, we attempt to create it.
        # Note: Create requires Organization ID and Project ID
        if not cfg.bw_organization_id:
            print(
                f"Error retrieving secret: {e}. Cannot create secret because BW_ORGANIZATION_ID is not set.",
                file=sys.stderr,
            )
            sys.exit(1)

        print(
            f"Secret '{cfg.bw_secret_id}' not found or error occurred. Creating a new secret...",
            file=sys.stderr,
        )
        try:
            project_ids = [cfg.bw_project_id] if cfg.bw_project_id else []
            new_secret = client.secrets().create(
                "Cottage Secrets",
                "Managed by Cottage",
                cfg.bw_organization_id,
                payload_str,
                project_ids,
            )
            print(
                f"Created new Bitwarden secret '{new_secret.key}' (ID: {new_secret.id})",
                file=sys.stderr,
            )
        except Exception as create_err:
            print(f"Error creating secret in Bitwarden: {create_err}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    app()
