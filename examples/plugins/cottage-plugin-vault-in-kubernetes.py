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
[upstream.dev-vault]
envfile = "./vault/dev.env.cott.age"
vars = {
  VAULT_MOUNT = "secrets",
  KUBE_NAMESPACE = "vault",
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

    @property
    def vault_secret_urlpath(self) -> str:
        return f"/v1/{self.vault_mount}/data/{self.vault_secret_path}"

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


@contextmanager
def kube_proxy_vault_client(config: VaultSecretConfig):
    if ":" in config.kube_port_forward:
        local_port, remote_port = map(int, config.kube_port_forward.split(":", 1))
    else:
        local_port = remote_port = int(config.kube_port_forward)

    with portforward.forward(
        namespace=config.kube_namespace,
        pod_or_service=config.kube_pod_or_service,
        from_port=local_port,
        to_port=remote_port,
        config_path=str(config.kube_config_path),
        kube_context=config.kube_context or "",
    ):
        print(  # Use --debug to see this message
            f"Port forwarding established: 127.0.0.1:{local_port} -> {config.kube_pod_or_service}:{remote_port}",
            file=sys.stderr,
        )
        with (
            SyncClientBuilder()
            .base_url(f"http://127.0.0.1:{local_port}")
            .default_headers({"X-Vault-Token": config.vault_token})
            .error_for_status()
            .build()
        ) as client:
            yield client


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = VaultSecretConfig()
    with kube_proxy_vault_client(cfg) as client:
        print(  # Use --debug to see this message
            "Pulling from", cfg.vault_secret_urlpath, file=sys.stderr
        )
        resp = client.get(cfg.vault_secret_urlpath).build().send()
    secret_data = resp.json()["data"]["data"]
    print("\n".join(f"{k}={repr(v)}" for k, v in secret_data.items()))


@app.command(name="push")
def cmd_push():
    cfg = VaultSecretConfig()
    payload = {"data": dotenv_values(stream=sys.stdin)}
    with kube_proxy_vault_client(cfg) as client:
        print(  # Use --debug to see this message
            "Pushing to", cfg.vault_secret_urlpath, file=sys.stderr
        )
        client.post(cfg.vault_secret_urlpath).body_json(payload).build().send()


if __name__ == "__main__":
    app()
