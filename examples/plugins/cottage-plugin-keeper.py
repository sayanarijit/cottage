#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "keeper-secrets-manager-core>=16.6.2",
# ]
# ///


# cottage.toml
"""
[upstream.dev-keeper]
envfile = "./keeper/dev.env.cott.age"  # Should export KSM_CONFIG (base64) or KEEPER_CONFIG_PATH.
vars = {
  KEEPER_RECORD_UID = "your-record-uid-here",
}
plugin = "./examples/plugins/cottage-plugin-keeper.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-keeper]
pull = true
push = true
"""

import json
import os
import sys
from pathlib import Path

from cyclopts import App
from keeper_secrets_manager_core import SecretsManager
from keeper_secrets_manager_core.storage import FileStorage
from pydantic import BaseModel, Field, model_validator


class KeeperSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    keeper_record_uid: str = Field(..., alias="KEEPER_RECORD_UID")
    keeper_config_path: Path | None = Field(None, alias="KEEPER_CONFIG_PATH")

    @model_validator(mode="after")
    def resolve_paths(self) -> "KeeperSecretConfig":
        if self.keeper_config_path:
            self.keeper_config_path = self.keeper_config_path.expanduser()
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


def get_keeper_client(config: KeeperSecretConfig) -> SecretsManager:
    if config.keeper_config_path:
        storage = FileStorage(str(config.keeper_config_path))
        return SecretsManager(storage=storage)
    else:
        # Automatically loads KSM_CONFIG from environment
        return SecretsManager()


app = App()


@app.command()
def pull():
    cfg = KeeperSecretConfig.model_validate(os.environ)
    client = get_keeper_client(cfg)
    print(  # Use --debug to see this message
        f"Retrieving record '{cfg.keeper_record_uid}' from Keeper Vault...",
        file=sys.stderr,
    )
    try:
        record = client.get_secrets([cfg.keeper_record_uid])[0]
    except Exception as e:
        print(f"Error retrieving record from Keeper: {e}", file=sys.stderr)
        sys.exit(1)

    # 1. Try to parse notes as JSON
    notes = record.notes
    if notes:
        try:
            data = json.loads(notes)
            print(json.dumps(data))
            return
        except json.JSONDecodeError:
            pass

    # 2. Try to parse password as JSON
    password = record.password
    if password:
        try:
            data = json.loads(password)
            print(json.dumps(data))
            return
        except json.JSONDecodeError:
            pass

    # 3. Fallback to assembling all fields
    secrets = {}
    if record.title:
        secrets["title"] = record.title
    if record.login:
        secrets["login"] = record.login
    if record.password:
        secrets["password"] = record.password
    if record.notes:
        secrets["notes"] = record.notes

    # Custom fields
    for field in getattr(record, "custom_fields", []):
        name = getattr(field, "name", None)
        value = getattr(field, "value", None)
        if name and value:
            secrets[name] = value[0] if isinstance(value, list) else value

    print(json.dumps(secrets))


@app.command()
def push():
    cfg = KeeperSecretConfig.model_validate(os.environ)
    client = get_keeper_client(cfg)
    payload = json.loads(input())
    payload_str = json.dumps(payload)

    print(  # Use --debug to see this message
        f"Updating record '{cfg.keeper_record_uid}' in Keeper Vault...",
        file=sys.stderr,
    )
    try:
        record = client.get_secrets([cfg.keeper_record_uid])[0]
        # We store the JSON payload in the password field or notes
        # Storing in notes is generally better for JSON payloads
        record.notes = payload_str
        client.save(record)
        print(f"Successfully saved record '{cfg.keeper_record_uid}'", file=sys.stderr)
    except Exception as e:
        print(f"Error updating record in Keeper: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    app()
