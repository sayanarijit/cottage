#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "keyring>=25.2.1",
# ]
# ///


# cottage.toml
"""
[upstream.dev-keyring]
vars = {
  KEYRING_SERVICE = "cottage-secrets",
  KEYRING_USERNAME = "myapp-dev",
}
plugin = "./examples/plugins/cottage-plugin-keyring.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-keyring]
pull = true
push = true
"""

import json
import os
import sys

import keyring
from cyclopts import App
from pydantic import BaseModel, Field


class KeyringSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    keyring_service: str = Field(..., alias="KEYRING_SERVICE")
    keyring_username: str = Field(..., alias="KEYRING_USERNAME")

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = KeyringSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        f"Retrieving password for service '{cfg.keyring_service}', username '{cfg.keyring_username}' from OS Keyring...",
        file=sys.stderr,
    )
    try:
        val = keyring.get_password(cfg.keyring_service, cfg.keyring_username)
    except Exception as e:
        print(f"Error accessing OS Keyring: {e}", file=sys.stderr)
        sys.exit(1)

    if val is None:
        print(
            f"No secret found in OS Keyring for service '{cfg.keyring_service}' and username '{cfg.keyring_username}'",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        data = json.loads(val)
        print(json.dumps(data))
    except json.JSONDecodeError:
        print(json.dumps({"value": val}))


@app.command(name="push")
def cmd_push():
    cfg = KeyringSecretConfig.model_validate(os.environ)
    payload_str = json.dumps(json.loads(input()))
    print(  # Use --debug to see this message
        f"Saving password for service '{cfg.keyring_service}', username '{cfg.keyring_username}' to OS Keyring...",
        file=sys.stderr,
    )
    try:
        keyring.set_password(cfg.keyring_service, cfg.keyring_username, payload_str)
    except Exception as e:
        print(f"Error writing to OS Keyring: {e}", file=sys.stderr)
        sys.exit(1)

    print("Successfully updated OS Keyring secret", file=sys.stderr)


if __name__ == "__main__":
    app()
