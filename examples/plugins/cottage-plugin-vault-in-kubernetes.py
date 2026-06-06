#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "dotenv>=0.9.9",
#     "portforward>=0.7.6",
#     "pydantic-settings>=2.14.1",
#     "pyreqwest>=0.10.1",
# ]
# ///


# cottage.toml
"""
[upstream.defaults]
vars = {
  KUBE_NAMESPACE = "vault",
  VAULT_MOUNT = "secrets",
}

[upstream.dev-vault]
envfile = "./vault/dev.env.cott.age"
vars = {
  KUBE_CONFIG_PATH = "./kubeconfig/dev.yaml",
}
plugin = "./plugins/cottage-plugin-vault-in-kubernetes.py"
"""

# vault/dev.env
"""
VAULT_TOKEN=dev-token
"""

# dockerconfigjson/dev.env.cott.toml
"""
[upstream.dev-vault]
pull = true
push = true

[upstream.dev-vault.vars]
VAULT_SECRET_PATH = "dockerconfigjson"
"""

import sys
from contextlib import contextmanager
from pathlib import Path

import portforward
from cyclopts import App
from dotenv import dotenv_values
from pydantic import Field
from pydantic_settings import BaseSettings
from pyreqwest.client import SyncClientBuilder


class VaultSecretConfig(BaseSettings):
    vault_token: str = Field(..., alias="VAULT_TOKEN", description="Pass via `envfile`")
    vault_mount: str = Field(..., alias="VAULT_MOUNT")
    vault_secret_path: str = Field(..., alias="VAULT_SECRET_PATH")
    kube_config_path: Path = Field(..., alias="KUBE_CONFIG_PATH")
    kube_context: str | None = Field(None, alias="KUBE_CONTEXT")
    kube_port_forward: str = Field("8200:8200", alias="KUBE_PORT_FORWARD")
    kube_namespace: str = Field("default", alias="KUBE_NAMESPACE")
    kube_pod_or_service: str = Field("vault", alias="KUBE_POD_OR_SERVICE")


@contextmanager
def kube_proxy_vault_client(config: VaultSecretConfig):
    if ":" in config.kube_port_forward:
        local_port, remote_port = map(int, config.kube_port_forward.split(":", 1))
    else:
        local_port = remote_port = int(config.kube_port_forward)

    base_url = f"http://localhost:{local_port}"
    with portforward.forward(
        namespace=config.kube_namespace,
        pod_or_service=config.kube_pod_or_service,
        from_port=local_port,
        to_port=remote_port,
        config_path=str(config.kube_config_path),
        kube_context=config.kube_context or "",
    ):
        with (
            SyncClientBuilder()
            .base_url(base_url)
            .default_headers({"X-Vault-Token": config.vault_token})
            .error_for_status()
            .build()
        ) as client:
            yield client


app = App()


@app.command(name="pull")
def cmd_pull():
    vault_config = VaultSecretConfig()
    remote_path = f"{vault_config.vault_mount}/data/{vault_config.vault_secret_path}"
    with kube_proxy_vault_client(vault_config) as client:
        resp = client.get(f"/v1/{remote_path}").build().send()
    secret_data = resp.json()["data"]["data"]
    print("\n".join(f"{k}={repr(v)}" for k, v in secret_data.items()))


@app.command(name="push")
def cmd_push():
    vault_config = VaultSecretConfig()
    remote_path = f"{vault_config.vault_mount}/data/{vault_config.vault_secret_path}"
    local_data = dotenv_values(stream=sys.stdin)
    payload = {"data": local_data}
    with kube_proxy_vault_client(vault_config) as client:
        client.post(f"/v1/{remote_path}").body_json(payload).build().send()


if __name__ == "__main__":
    app()
