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
[upstream.dev-passhole]
vars = {
  PASSHOLE_SECRET_PATH = "myapp/dev/secrets",
}
plugin = "./examples/plugins/cottage-plugin-passhole.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-passhole]
pull = true
"""

import json
import os
import subprocess
import sys

from pathlib import Path

from cyclopts import App
from pydantic import BaseModel, Field, model_validator


class PassholeSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    passhole_secret_path: str = Field(..., alias="PASSHOLE_SECRET_PATH")
    passhole_bin_path: Path = Field(Path("ph"), alias="PASSHOLE_BIN_PATH")

    @model_validator(mode="after")
    def resolve_paths(self) -> "PassholeSecretConfig":
        self.passhole_bin_path = self.passhole_bin_path.expanduser()
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command()
def pull():
    cfg = PassholeSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        f"Pulling secret '{cfg.passhole_secret_path}' from Passhole...",
        file=sys.stderr,
    )
    try:
        res = subprocess.run(
            [cfg.passhole_bin_path, "show", cfg.passhole_secret_path],
            capture_output=True,
            text=True,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        print(
            f"Error running '{cfg.passhole_bin_path} show': {e.stderr}", file=sys.stderr
        )
        sys.exit(1)

    output = res.stdout.strip()

    # Try to parse entire output as JSON
    try:
        data = json.loads(output)
        print(json.dumps(data))
        return
    except json.JSONDecodeError:
        pass

    # Parse key-value lines: Title: name, Password: val, etc.
    lines = output.splitlines()
    data = {}
    for line in lines:
        if ":" in line:
            k, v = line.split(":", 1)
            data[k.strip().lower()] = v.strip()
    print(json.dumps(data))


@app.command()
def push():
    cfg = PassholeSecretConfig.model_validate(os.environ)
    print(
        f"Error: The Passhole CLI ('{cfg.passhole_bin_path}') is designed for interactive database management and "
        "does not support non-interactive updates. Please update KeePass databases using keepassxc-cli "
        "or write changes manually using KeePass apps.",
        file=sys.stderr,
    )
    sys.exit(1)


if __name__ == "__main__":
    app()
