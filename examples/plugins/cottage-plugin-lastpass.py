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
[upstream.dev-lastpass]
envfile = "./lastpass/dev.env.cott.age"  # Should export LPASS_AGENT_KEY or authenticate session.
vars = {
  LPASS_SECRET_NAME = "myapp/dev/secrets",
}
plugin = "./examples/plugins/cottage-plugin-lastpass.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-lastpass]
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


class LastPassSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    lpass_secret_name: str = Field(..., alias="LPASS_SECRET_NAME")
    lpass_bin_path: Path = Field(Path("lpass"), alias="LPASS_BIN_PATH")

    @model_validator(mode="after")
    def resolve_paths(self) -> "LastPassSecretConfig":
        self.lpass_bin_path = self.lpass_bin_path.expanduser()
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = LastPassSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        f"Pulling notes for secret '{cfg.lpass_secret_name}' from LastPass...",
        file=sys.stderr,
    )
    try:
        # Retrieve notes field
        res = subprocess.run(
            [cfg.lpass_bin_path, "show", "--notes", cfg.lpass_secret_name],
            capture_output=True,
            text=True,
            check=True,
        )
        notes = res.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(
            f"Error running '{cfg.lpass_bin_path} show --notes': {e.stderr}",
            file=sys.stderr,
        )
        sys.exit(1)

    # Try to parse notes as JSON
    try:
        data = json.loads(notes)
        print(json.dumps(data))
        return
    except json.JSONDecodeError:
        pass

    # Fallback: fetch other standard fields
    secrets = {}
    if notes:
        secrets["notes"] = notes
    try:
        # Get password
        pwd_res = subprocess.run(
            [cfg.lpass_bin_path, "show", "--password", cfg.lpass_secret_name],
            capture_output=True,
            text=True,
        )
        if pwd_res.returncode == 0 and pwd_res.stdout.strip():
            secrets["password"] = pwd_res.stdout.strip()

        # Get username
        user_res = subprocess.run(
            [cfg.lpass_bin_path, "show", "--username", cfg.lpass_secret_name],
            capture_output=True,
            text=True,
        )
        if user_res.returncode == 0 and user_res.stdout.strip():
            secrets["username"] = user_res.stdout.strip()
    except Exception:
        pass

    print(json.dumps(secrets))


@app.command(name="push")
def cmd_push():
    cfg = LastPassSecretConfig.model_validate(os.environ)
    payload = json.loads(input())

    # Store JSON in Notes or password
    if "password" in payload and len(payload) == 1:
        # If it only contains password, write directly to password field
        field_to_edit = "--password"
        content = str(payload["password"])
    else:
        # Otherwise store the entire JSON object in notes
        field_to_edit = "--notes"
        content = json.dumps(payload)

    print(  # Use --debug to see this message
        f"Pushing secret to LastPass at '{cfg.lpass_secret_name}'...",
        file=sys.stderr,
    )

    try:
        # lpass edit takes --non-interactive and reads from stdin
        subprocess.run(
            [
                cfg.lpass_bin_path,
                "edit",
                "--non-interactive",
                field_to_edit,
                cfg.lpass_secret_name,
            ],
            input=content,
            text=True,
            check=True,
            capture_output=True,
        )
    except subprocess.CalledProcessError as e:
        print(f"Error running '{cfg.lpass_bin_path} edit': {e.stderr}", file=sys.stderr)
        sys.exit(1)

    print(
        f"Successfully updated LastPass secret '{cfg.lpass_secret_name}'",
        file=sys.stderr,
    )


if __name__ == "__main__":
    app()
