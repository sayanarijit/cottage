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
[upstream.dev-pass]
vars = {
  PASS_SECRET_PATH = "myapp/dev/secrets",
  PASSWORD_STORE_DIR = "~/.password-store",  # Optional override
}
plugin = "./examples/plugins/cottage-plugin-pass.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-pass]
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


class PassSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    pass_secret_path: str = Field(..., alias="PASS_SECRET_PATH")
    password_store_dir: Path | None = Field(None, alias="PASSWORD_STORE_DIR")
    pass_bin_path: Path = Field(Path("pass"), alias="PASS_BIN_PATH")
    gpg_passphrase: str | None = Field(None, alias="GPG_PASSPHRASE")

    @model_validator(mode="after")
    def resolve_paths(self) -> "PassSecretConfig":
        self.pass_bin_path = self.pass_bin_path.expanduser()
        if self.password_store_dir:
            self.password_store_dir = self.password_store_dir.expanduser()
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command()
def pull():
    cfg = PassSecretConfig.model_validate(os.environ)
    print(  # Use --debug to see this message
        f"Pulling secret '{cfg.pass_secret_path}' from GNU pass...",
        file=sys.stderr,
    )
    env = os.environ.copy()
    if cfg.password_store_dir:
        env["PASSWORD_STORE_DIR"] = str(cfg.password_store_dir)
    if cfg.gpg_passphrase:
        existing_opts = env.get("PASSWORD_STORE_GPG_OPTS", "")
        passphrase_opts = f"--pinentry-mode loopback --passphrase {cfg.gpg_passphrase}"
        env["PASSWORD_STORE_GPG_OPTS"] = f"{existing_opts} {passphrase_opts}".strip()

    try:
        res = subprocess.run(
            [cfg.pass_bin_path, "show", cfg.pass_secret_path],
            capture_output=True,
            text=True,
            env=env,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        print(f"Error running '{cfg.pass_bin_path} show': {e.stderr}", file=sys.stderr)
        sys.exit(1)

    output = res.stdout.strip()

    # Try to parse entire output as JSON
    try:
        data = json.loads(output)
        print(json.dumps(data))
        return
    except json.JSONDecodeError:
        pass

    # Fallback to standard pass multi-line format parsing
    # First line is password, subsequent lines are key-value metadata (key: value)
    lines = output.splitlines()
    data = {}
    if lines:
        data["password"] = lines[0].strip()
        for line in lines[1:]:
            if ":" in line:
                k, v = line.split(":", 1)
                data[k.strip()] = v.strip()
    print(json.dumps(data))


@app.command()
def push():
    cfg = PassSecretConfig.model_validate(os.environ)
    payload = json.loads(input())

    # Format payload for pass
    if "password" in payload:
        lines = [str(payload["password"])]
        for k, v in payload.items():
            if k != "password":
                lines.append(f"{k}: {v}")
        content = "\n".join(lines)
    else:
        content = json.dumps(payload)

    print(  # Use --debug to see this message
        f"Pushing secret to GNU pass at '{cfg.pass_secret_path}'...",
        file=sys.stderr,
    )

    env = os.environ.copy()
    if cfg.password_store_dir:
        env["PASSWORD_STORE_DIR"] = str(cfg.password_store_dir)
    if cfg.gpg_passphrase:
        existing_opts = env.get("PASSWORD_STORE_GPG_OPTS", "")
        passphrase_opts = f"--pinentry-mode loopback --passphrase {cfg.gpg_passphrase}"
        env["PASSWORD_STORE_GPG_OPTS"] = f"{existing_opts} {passphrase_opts}".strip()

    try:
        subprocess.run(
            [cfg.pass_bin_path, "insert", "-mf", cfg.pass_secret_path],
            input=content,
            text=True,
            env=env,
            check=True,
            capture_output=True,
        )
    except subprocess.CalledProcessError as e:
        print(
            f"Error running '{cfg.pass_bin_path} insert': {e.stderr}", file=sys.stderr
        )
        sys.exit(1)


if __name__ == "__main__":
    app()
