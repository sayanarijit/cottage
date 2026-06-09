#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
# ]
# ///


# cottage.toml
"""
[upstream.dev-protonpass]
vars = {
  PROTON_ITEM_TITLE = "myapp-dev-secrets",
  PROTON_VAULT_NAME = "My Vault",  # Optional
}
plugin = "./examples/plugins/cottage-plugin-protonpass.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-protonpass]
pull = true
push = true
"""

import json
import os
import subprocess
import sys

from pathlib import Path

from cyclopts import App
from pydantic import BaseModel, Field, model_validator


class ProtonPassSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    proton_item_title: str = Field(..., alias="PROTON_ITEM_TITLE")
    proton_vault_name: str | None = Field(None, alias="PROTON_VAULT_NAME")
    proton_bin_path: Path = Field(Path("pass-cli"), alias="PROTON_BIN_PATH")

    @model_validator(mode="after")
    def resolve_paths(self) -> "ProtonPassSecretConfig":
        self.proton_bin_path = self.proton_bin_path.expanduser()
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = ProtonPassSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        f"Pulling item '{cfg.proton_item_title}' from Proton Pass...",
        file=sys.stderr,
    )

    cmd = [
        cfg.proton_bin_path,
        "pass",
        "items",
        "view",
        cfg.proton_item_title,
        "--output",
        "json",
    ]
    if cfg.proton_vault_name:
        cmd.extend(["--vault", cfg.proton_vault_name])

    try:
        res = subprocess.run(cmd, capture_output=True, text=True, check=True)
        item_data = json.loads(res.stdout)
    except subprocess.CalledProcessError as e:
        print(
            f"Error running '{cfg.proton_bin_path} pass items view': {e.stderr}",
            file=sys.stderr,
        )
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Failed to parse Proton Pass CLI output: {e}", file=sys.stderr)
        sys.exit(1)

    # Proton Pass item JSON structure contains fields or notes
    # Let's extract the notes field first in case it has our JSON payload
    note = item_data.get("note", "") or item_data.get("notes", "")
    if note:
        try:
            data = json.loads(note)
            print(json.dumps(data))
            return
        except json.JSONDecodeError:
            pass

    # Extract other fields
    secrets = {}
    if note:
        secrets["note"] = note
    if "password" in item_data:
        secrets["password"] = item_data["password"]
    if "username" in item_data:
        secrets["username"] = item_data["username"]
    if "email" in item_data:
        secrets["email"] = item_data["email"]

    # Also check fields list if it is structured
    for field in item_data.get("fields", []):
        name = field.get("name")
        value = field.get("value")
        if name and value:
            secrets[name] = value

    print(json.dumps(secrets))


@app.command(name="push")
def cmd_push():
    cfg = ProtonPassSecretConfig.model_validate(os.environ)
    payload = json.loads(input())
    content = json.dumps(payload)

    print(  # Use --debug to see this message
        f"Pushing item '{cfg.proton_item_title}' to Proton Pass...",
        file=sys.stderr,
    )

    # First check if the item exists
    check_cmd = [cfg.proton_bin_path, "pass", "items", "view", cfg.proton_item_title]
    if cfg.proton_vault_name:
        check_cmd.extend(["--vault", cfg.proton_vault_name])

    res = subprocess.run(check_cmd, capture_output=True)
    item_exists = res.returncode == 0

    if item_exists:
        # Edit existing item's note
        cmd = [
            cfg.proton_bin_path,
            "pass",
            "items",
            "edit",
            cfg.proton_item_title,
            "--note",
            content,
        ]
        if cfg.proton_vault_name:
            cmd.extend(["--vault", cfg.proton_vault_name])
    else:
        # Create a new secure note item
        cmd = [
            cfg.proton_bin_path,
            "pass",
            "items",
            "create",
            "--type",
            "note",
            "--name",
            cfg.proton_item_title,
            "--note",
            content,
        ]
        if cfg.proton_vault_name:
            cmd.extend(["--vault", cfg.proton_vault_name])

    try:
        subprocess.run(cmd, check=True, capture_output=True)
        print(f"Successfully saved item '{cfg.proton_item_title}'", file=sys.stderr)
    except subprocess.CalledProcessError as e:
        print(f"Error updating Proton Pass item: {e.stderr}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    app()
