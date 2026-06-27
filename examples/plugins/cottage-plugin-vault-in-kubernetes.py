#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "portforward>=0.7.6",
#     "pydantic>=2.13.4",
#     "pyreqwest>=0.10.1",
# ]
# ///


# cottage.toml
"""
[upstream.dev-vault]
envfile = "./vault/dev.env.cott.age"  # Should export VAULT_TOKEN.
vars = {
  VAULT_MOUNT = "secret",
  KUBE_NAMESPACE = "vault",
  KUBE_CONFIG_PATH = "./kubeconfig/dev.yaml",
}
plugin = "./plugins/cottage-plugin-vault-in-kubernetes.py"
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
from pathlib import Path

import portforward
from cyclopts import App
from pydantic import BaseModel, Field, model_validator
from pyreqwest.client import SyncClientBuilder


class VaultSecretConfig(BaseModel):
    vault_token: str = Field(..., alias="VAULT_TOKEN", description="Pass via `envfile`")
    vault_mount: str = Field(..., alias="VAULT_MOUNT")
    vault_secret_path: str = Field(..., alias="VAULT_SECRET_PATH")
    vault_namespace: str | None = Field(None, alias="VAULT_NAMESPACE")
    kube_config_path: Path = Field(..., alias="KUBE_CONFIG_PATH")
    kube_context: str | None = Field(None, alias="KUBE_CONTEXT")
    kube_port_forward: str = Field("8200:8200", alias="KUBE_PORT_FORWARD")
    kube_namespace: str = Field("default", alias="KUBE_NAMESPACE")
    kube_pod_or_service: str = Field("vault", alias="KUBE_POD_OR_SERVICE")

    @model_validator(mode="after")
    def resolve_paths(self) -> "VaultSecretConfig":
        if self.kube_config_path:
            self.kube_config_path = self.kube_config_path.expanduser()
        return self

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
            .default_headers(config.vault_headers)
            .error_for_status()
            .build()
        ) as client:
            yield client


app = App()


@app.command()
def pull():
    cfg = VaultSecretConfig.model_validate(os.environ)
    with kube_proxy_vault_client(cfg) as client:
        print(  # Use --debug to see this message
            "Pulling from", cfg.vault_secret_urlpath, file=sys.stderr
        )
        resp = client.get(cfg.vault_secret_urlpath).build().send()
    print(json.dumps(resp.json()["data"]["data"]))


@app.command()
def push():
    cfg = VaultSecretConfig.model_validate(os.environ)
    payload = {"data": json.loads(input())}
    with kube_proxy_vault_client(cfg) as client:
        print(  # Use --debug to see this message
            "Pushing to", cfg.vault_secret_urlpath, file=sys.stderr
        )
        client.post(cfg.vault_secret_urlpath).body_json(payload).build().send()


if __name__ == "__main__":
    app()
