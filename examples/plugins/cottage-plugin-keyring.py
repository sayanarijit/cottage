#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
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
from pydantic import BaseModel, Field


class KeyringSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    keyring_service: str = Field(..., alias="KEYRING_SERVICE")
    keyring_username: str = Field(..., alias="KEYRING_USERNAME")


def pull():
    cfg = KeyringSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        "Retrieving secret from OS Keyring...",
        file=sys.stderr,
    )
    try:
        val = keyring.get_password(cfg.keyring_service, cfg.keyring_username)
    except Exception as e:
        print(f"Error accessing OS Keyring: {e}", file=sys.stderr)
        sys.exit(1)

    if val is None:
        print(
            "No secret found in OS Keyring",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        data = json.loads(val)
        print(json.dumps(data))
    except json.JSONDecodeError:
        print(json.dumps({"value": val}))


def push():
    cfg = KeyringSecretConfig.model_validate(os.environ)
    payload_str = json.dumps(json.loads(input()))
    print(  # Use --debug to see this message
        "Saving secret to OS Keyring...",
        file=sys.stderr,
    )
    try:
        keyring.set_password(cfg.keyring_service, cfg.keyring_username, payload_str)
    except Exception as e:
        print(f"Error writing to OS Keyring: {e}", file=sys.stderr)
        sys.exit(1)

    print("Successfully updated OS Keyring secret", file=sys.stderr)


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
