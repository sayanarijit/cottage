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
[upstream.dev-dashlane]
vars = {
  DASHLANE_SECRET_ID = "your-secret-uuid-here",
}
plugin = "./examples/plugins/cottage-plugin-dashlane.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-dashlane]
pull = true
"""

import json
import os
import subprocess
import sys

from pathlib import Path

from cyclopts import App
from pydantic import BaseModel, Field, model_validator


class DashlaneSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    dashlane_secret_id: str = Field(..., alias="DASHLANE_SECRET_ID")
    dashlane_bin_path: Path = Field(Path("dcli"), alias="DASHLANE_BIN_PATH")

    @model_validator(mode="after")
    def resolve_paths(self) -> "DashlaneSecretConfig":
        self.dashlane_bin_path = self.dashlane_bin_path.expanduser()
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = DashlaneSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        f"Pulling secret '{cfg.dashlane_secret_id}' via Dashlane CLI (dcli)...",
        file=sys.stderr,
    )
    uri = f"dl://{cfg.dashlane_secret_id}"
    try:
        res = subprocess.run(
            [cfg.dashlane_bin_path, "read", uri],
            capture_output=True,
            text=True,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        print(
            f"Error running Dashlane CLI ({cfg.dashlane_bin_path}): {e.stderr}",
            file=sys.stderr,
        )
        sys.exit(1)

    output = res.stdout.strip()
    try:
        data = json.loads(output)
        print(json.dumps(data))
    except json.JSONDecodeError:
        print(json.dumps({"value": output}))


@app.command(name="push")
def cmd_push():
    cfg = DashlaneSecretConfig.model_validate(os.environ)
    print(
        f"Error: The Dashlane CLI ('{cfg.dashlane_bin_path}') does not support writing/creating secrets directly. "
        "Please manage your secrets via the Dashlane web application or browser extension.",
        file=sys.stderr,
    )
    sys.exit(1)


if __name__ == "__main__":
    app()
