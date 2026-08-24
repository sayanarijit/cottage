#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "pydantic>=2.13.4",
#     "pyreqwest>=0.10.1",
# ]
# ///


# cottage.toml
"""
[upstream.dev-doppler]
envfile = "./doppler/dev.env.cott.age"  # Should export DOPPLER_TOKEN.
vars = {
  DOPPLER_PROJECT = "my-project",
  DOPPLER_CONFIG = "dev",
}
plugin = "./examples/plugins/cottage-plugin-doppler.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-doppler]
pull = true
push = true
"""

import json
import os
import sys
from contextlib import contextmanager

from pydantic import BaseModel, Field
from pyreqwest.client import SyncClientBuilder


class DopplerSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    doppler_token: str = Field(..., alias="DOPPLER_TOKEN")
    doppler_project: str = Field(..., alias="DOPPLER_PROJECT")
    doppler_config: str = Field(..., alias="DOPPLER_CONFIG")

    @property
    def doppler_headers(self) -> dict[str, str]:
        return {
            "Authorization": f"Bearer {self.doppler_token}",
            "Accept": "application/json",
        }


@contextmanager
def doppler_client(config: DopplerSecretConfig):
    with (
        SyncClientBuilder()
        .base_url("https://api.doppler.com")
        .default_headers(config.doppler_headers)
        .error_for_status()
        .build()
    ) as client:
        yield client


def pull():
    cfg = DopplerSecretConfig.model_validate(os.environ)
    with doppler_client(cfg) as client:
        urlpath = f"/v3/configs/config/secrets/download?project={cfg.doppler_project}&config={cfg.doppler_config}&format=json"
        print(  # Use --debug to see this message
            "Pulling secrets from Doppler...",
            file=sys.stderr,
        )
        resp = client.get(urlpath).build().send()
    print(json.dumps(resp.json()))


def push():
    cfg = DopplerSecretConfig.model_validate(os.environ)
    secrets_data = json.loads(input())
    payload = {
        "project": cfg.doppler_project,
        "config": cfg.doppler_config,
        "secrets": secrets_data,
    }
    with doppler_client(cfg) as client:
        print(  # Use --debug to see this message
            "Pushing secrets to Doppler...",
            file=sys.stderr,
        )
        client.post("/v3/configs/config/secrets").body_json(payload).build().send()


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} [pull|push]", file=sys.stderr)
        sys.exit(1)

    match sys.argv[1]:
        case "pull":
            pull()
        case "push":
            push()
        case cmd:
            print(f"Unknown command: {cmd}", file=sys.stderr)
            sys.exit(1)
