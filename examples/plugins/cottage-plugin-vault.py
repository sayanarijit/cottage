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
[upstream.dev-vault]
envfile = "./vault/dev.env.cott.age"  # Should export VAULT_TOKEN.
vars = {
  VAULT_ADDR = "http://localhost:8200",
  VAULT_MOUNT = "secret",
}
plugin = "./plugins/cottage-plugin-vault.py"
"""

# HCP Vault Dedicated / Enterprise namespaces
"""
[upstream.org-vault]
envfile = "./vault/org.env.cott.age"  # Should export VAULT_TOKEN.
vars = {
  VAULT_ADDR = "https://cluster-id.hashicorp.cloud:8200",
  VAULT_NAMESPACE = "admin",
  VAULT_MOUNT = "secret",
}
plugin = "./plugins/cottage-plugin-vault.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-vault]
pull = true
push = true

[upstream.dev-vault.vars]
VAULT_SECRET_PATH = "myapp/env/dev"
"""

import json
import os
import sys
from contextlib import contextmanager

from cyclopts import App
from pydantic import BaseModel, Field
from pyreqwest.client import SyncClientBuilder


class VaultSecretConfig(BaseModel):
    vault_addr: str = Field(..., alias="VAULT_ADDR")
    vault_token: str = Field(..., alias="VAULT_TOKEN", description="Pass via `envfile`")
    vault_mount: str = Field(..., alias="VAULT_MOUNT")
    vault_secret_path: str = Field(..., alias="VAULT_SECRET_PATH")
    vault_namespace: str | None = Field(None, alias="VAULT_NAMESPACE")

    @property
    def vault_base_url(self) -> str:
        return self.vault_addr.rstrip("/")

    @property
    def vault_secret_urlpath(self) -> str:
        return f"/v1/{self.vault_mount}/data/{self.vault_secret_path}"

    @property
    def vault_headers(self) -> dict[str, str]:
        headers = {
            "X-Vault-Request": "true",
            "X-Vault-Token": self.vault_token,
        }
        if self.vault_namespace:
            headers["X-Vault-Namespace"] = self.vault_namespace
        return headers

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


@contextmanager
def vault_client(config: VaultSecretConfig):
    with (
        SyncClientBuilder()
        .base_url(config.vault_base_url)
        .default_headers(config.vault_headers)
        .error_for_status()
        .build()
    ) as client:
        yield client


app = App()


@app.command()
def pull():
    cfg = VaultSecretConfig.model_validate(os.environ)
    with vault_client(cfg) as client:
        print(  # Use --debug to see this message
            "Pulling from", cfg.vault_secret_urlpath, file=sys.stderr
        )
        resp = client.get(cfg.vault_secret_urlpath).build().send()
    print(json.dumps(resp.json()["data"]["data"]))


@app.command()
def push():
    cfg = VaultSecretConfig.model_validate(os.environ)
    payload = {"data": json.loads(input())}
    with vault_client(cfg) as client:
        print(  # Use --debug to see this message
            "Pushing to", cfg.vault_secret_urlpath, file=sys.stderr
        )
        client.post(cfg.vault_secret_urlpath).body_json(payload).build().send()


if __name__ == "__main__":
    app()
