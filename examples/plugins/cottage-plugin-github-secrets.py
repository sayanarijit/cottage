#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "pydantic>=2.13.4",
# ]
# ///

from __future__ import annotations

# cottage.toml
"""
[upstream.dev-github]
vars = {
  GH_REPO = "owner/repo",
  GH_APP = "actions",  # actions | agents | codespaces | dependabot
}
plugin = "./examples/plugins/cottage-plugin-github-secrets.py"

# Or for organization secrets:
[upstream.org-github]
vars = {
  GH_ORG = "my-org",
  GH_VISIBILITY = "selected",  # all | private | selected
  GH_REPOS = "repo1,repo2",
}
plugin = "./examples/plugins/cottage-plugin-github-secrets.py"

# Or for deployment environment secrets:
[upstream.env-github]
vars = {
  GH_REPO = "owner/repo",
  GH_ENV = "production",
}
plugin = "./examples/plugins/cottage-plugin-github-secrets.py"

# Or for user secrets:
[upstream.user-github]
vars = {
  GH_USER = "true",
  GH_REPOS = "repo1,repo2",
}
plugin = "./examples/plugins/cottage-plugin-github-secrets.py"
"""

# myapp/dev.env.cott.toml
"""
[upstream.dev-github]
push = true
"""

import json
import os
import subprocess
import sys
from pathlib import Path

from pydantic import AliasChoices, BaseModel, Field, field_validator, model_validator


class GitHubSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}

    gh_bin_path: Path = Field(
        Path("gh"),
        validation_alias=AliasChoices("GH_BIN_PATH", "GITHUB_BIN_PATH"),
    )
    gh_app: str | None = Field(
        None,
        validation_alias=AliasChoices("GH_APP", "GITHUB_APP"),
    )
    gh_env: str | None = Field(
        None,
        validation_alias=AliasChoices("GH_ENV", "GH_ENVIRONMENT", "GITHUB_ENVIRONMENT"),
    )
    gh_no_repos_selected: bool = Field(
        False,
        validation_alias=AliasChoices(
            "GH_NO_REPOS_SELECTED", "GITHUB_NO_REPOS_SELECTED"
        ),
    )
    gh_org: str | None = Field(
        None,
        validation_alias=AliasChoices(
            "GH_ORG", "GITHUB_ORG", "GH_ORGANIZATION", "GITHUB_ORGANIZATION"
        ),
    )
    gh_repos: str | None = Field(
        None,
        validation_alias=AliasChoices(
            "GH_REPOS", "GITHUB_REPOS", "GH_REPOSITORIES", "GITHUB_REPOSITORIES"
        ),
    )
    gh_user: bool = Field(
        False,
        validation_alias=AliasChoices("GH_USER", "GITHUB_USER"),
    )
    gh_visibility: str | None = Field(
        None,
        validation_alias=AliasChoices("GH_VISIBILITY", "GITHUB_VISIBILITY"),
    )
    gh_repo: str | None = Field(
        None,
        validation_alias=AliasChoices(
            "GH_REPO", "GITHUB_REPO", "GH_REPOSITORY", "GITHUB_REPOSITORY"
        ),
    )

    @field_validator("gh_no_repos_selected", "gh_user", mode="before")
    @classmethod
    def empty_str_to_false(cls, v: object) -> object:
        if v == "":
            return False
        return v

    @field_validator(
        "gh_app",
        "gh_env",
        "gh_org",
        "gh_repos",
        "gh_visibility",
        "gh_repo",
        mode="before",
    )
    @classmethod
    def empty_str_to_none(cls, v: object) -> object:
        if v == "":
            return None
        return v

    @model_validator(mode="after")
    def resolve_paths(self) -> GitHubSecretConfig:
        self.gh_bin_path = self.gh_bin_path.expanduser()
        return self


def pull() -> None:
    print(
        "Error: GitHub Secrets are write-only and cannot be retrieved via the GitHub CLI or API.",
        file=sys.stderr,
    )
    sys.exit(1)


def push() -> None:
    cfg = GitHubSecretConfig.model_validate(os.environ)
    raw_input = sys.stdin.read()

    # If the input is valid JSON mapping, convert to dotenv format
    try:
        data = json.loads(raw_input)
        if isinstance(data, dict):
            lines = []
            for k, v in data.items():
                if isinstance(v, (str, int, float, bool)):
                    lines.append(f"{k}={v}")
                else:
                    lines.append(f"{k}={json.dumps(v)}")
            content = "\n".join(lines) + "\n"
        else:
            content = raw_input
    except (json.JSONDecodeError, TypeError):
        content = raw_input

    cmd: list[str] = [str(cfg.gh_bin_path), "secret", "set", "-f", "-"]

    if cfg.gh_app:
        cmd.extend(["--app", cfg.gh_app])
    if cfg.gh_env:
        cmd.extend(["--env", cfg.gh_env])
    if cfg.gh_no_repos_selected:
        cmd.append("--no-repos-selected")
    if cfg.gh_org:
        cmd.extend(["--org", cfg.gh_org])
    if cfg.gh_repos:
        cmd.extend(["--repos", cfg.gh_repos])
    if cfg.gh_user:
        cmd.append("--user")
    if cfg.gh_visibility:
        cmd.extend(["--visibility", cfg.gh_visibility])
    if cfg.gh_repo:
        cmd.extend(["--repo", cfg.gh_repo])

    print(  # Use --debug to see this message
        "Pushing secrets to GitHub...",
        file=sys.stderr,
    )

    try:
        res = subprocess.run(
            cmd,
            input=content,
            text=True,
            capture_output=True,
            check=True,
        )
        if res.stdout:
            print(res.stdout.strip(), file=sys.stderr)
    except subprocess.CalledProcessError as e:
        err_msg = e.stderr.strip() or e.stdout.strip() or str(e)
        print(
            f"Error running '{cfg.gh_bin_path} secret set': {err_msg}",
            file=sys.stderr,
        )
        sys.exit(1)

    print("Successfully pushed secrets to GitHub", file=sys.stderr)


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
