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
[upstream.dev-ejson]
vars = {
  EJSON_PATH = "./examples/secrets.ejson",
}
plugin = "./examples/plugins/cottage-plugin-ejson.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-ejson]
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


class EjsonSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    ejson_path: Path = Field(..., alias="EJSON_PATH")
    ejson_bin_path: Path = Field(Path("ejson"), alias="EJSON_BIN_PATH")

    @model_validator(mode="after")
    def resolve_paths(self) -> "EjsonSecretConfig":
        if self.ejson_path:
            self.ejson_path = self.ejson_path.expanduser()
        self.ejson_bin_path = self.ejson_bin_path.expanduser()
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command(name="pull")
def cmd_pull():
    cfg = EjsonSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        f"Decrypting EJSON file '{cfg.ejson_path}'...",
        file=sys.stderr,
    )
    if not cfg.ejson_path.exists():
        print(f"Error: EJSON file '{cfg.ejson_path}' does not exist.", file=sys.stderr)
        sys.exit(1)

    try:
        res = subprocess.run(
            [cfg.ejson_bin_path, "decrypt", str(cfg.ejson_path)],
            capture_output=True,
            text=True,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        print(
            f"Error decrypting EJSON file ({cfg.ejson_bin_path}): {e.stderr}",
            file=sys.stderr,
        )
        sys.exit(1)

    print(res.stdout)


@app.command(name="push")
def cmd_push():
    cfg = EjsonSecretConfig.model_validate(os.environ)
    payload = json.loads(input())

    print(  # Use --debug to see this message
        f"Encrypting secrets to EJSON file '{cfg.ejson_path}'...",
        file=sys.stderr,
    )

    # We want to preserve '_public_key' and other metadata starting with '_' from the existing file if it exists
    metadata = {}
    if cfg.ejson_path.exists():
        try:
            with open(cfg.ejson_path) as f:
                existing = json.load(f)
                for k, v in existing.items():
                    if k.startswith("_"):
                        metadata[k] = v
        except Exception as e:
            print(
                f"Warning: Could not parse existing EJSON file metadata: {e}",
                file=sys.stderr,
            )

    # If _public_key is missing, EJSON cannot encrypt
    if "_public_key" not in metadata and "_public_key" not in payload:
        print(
            "Error: '_public_key' field is missing. Please define it in your EJSON file or secrets payload.",
            file=sys.stderr,
        )
        sys.exit(1)

    # Merge metadata into the payload
    for k, v in metadata.items():
        payload[k] = v

    # Write merged plain JSON to file
    try:
        with open(cfg.ejson_path, "w") as f:
            json.dump(payload, f, indent=2)
    except Exception as e:
        print(f"Error writing to EJSON file: {e}", file=sys.stderr)
        sys.exit(1)

    # Run ejson encrypt to encrypt in place
    try:
        subprocess.run(
            [cfg.ejson_bin_path, "encrypt", str(cfg.ejson_path)],
            capture_output=True,
            text=True,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        print(
            f"Error running '{cfg.ejson_bin_path} encrypt': {e.stderr}", file=sys.stderr
        )
        sys.exit(1)

    print(f"Successfully encrypted and updated '{cfg.ejson_path}'", file=sys.stderr)


if __name__ == "__main__":
    app()
