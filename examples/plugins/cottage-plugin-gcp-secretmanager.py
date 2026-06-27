#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "google-cloud-secret-manager>=2.20.0",
# ]
# ///


# cottage.toml
"""
[upstream.dev-gcp]
envfile = "./gcp/dev.env.cott.age"  # Should export GOOGLE_APPLICATION_CREDENTIALS.
vars = {
  GCP_PROJECT = "my-gcp-project",
  GCP_SECRET_ID = "myapp-dev-secrets",
}
plugin = "./examples/plugins/cottage-plugin-gcp-secretmanager.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-gcp]
pull = true
push = true
"""

import json
import os
import sys

from cyclopts import App
from google.cloud import secretmanager
from pydantic import BaseModel, Field


class GCPSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    gcp_project: str = Field(..., alias="GCP_PROJECT")
    gcp_secret_id: str = Field(..., alias="GCP_SECRET_ID")
    gcp_secret_version: str = Field("latest", alias="GCP_SECRET_VERSION")

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


app = App()


@app.command()
def pull():
    cfg = GCPSecretConfig.model_validate(os.environ)
    client = secretmanager.SecretManagerServiceClient()
    name = f"projects/{cfg.gcp_project}/secrets/{cfg.gcp_secret_id}/versions/{cfg.gcp_secret_version}"
    print(  # Use --debug to see this message
        f"Pulling secret version '{name}' from GCP Secret Manager...",
        file=sys.stderr,
    )
    try:
        response = client.access_secret_version(request={"name": name})
        payload = response.payload.data.decode("UTF-8")
    except Exception as e:
        print(f"Error accessing secret version from GCP: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        # Try to return as JSON if it's a JSON object
        data = json.loads(payload)
        print(json.dumps(data))
    except json.JSONDecodeError:
        # Fallback to returning the raw string wrapped in a dict
        print(json.dumps({"value": payload}))


@app.command()
def push():
    cfg = GCPSecretConfig.model_validate(os.environ)
    client = secretmanager.SecretManagerServiceClient()
    payload = json.dumps(json.loads(input())).encode("UTF-8")
    parent = f"projects/{cfg.gcp_project}/secrets/{cfg.gcp_secret_id}"
    print(  # Use --debug to see this message
        f"Pushing secret to GCP Secret Manager at '{parent}'...",
        file=sys.stderr,
    )

    try:
        client.add_secret_version(
            request={"parent": parent, "payload": {"data": payload}}
        )
    except Exception as e:
        # If the secret does not exist, attempt to create it first
        if "not found" in str(e).lower() or "notfound" in str(e).lower():
            print(
                f"Secret '{cfg.gcp_secret_id}' not found. Creating it...",
                file=sys.stderr,
            )
            project_path = f"projects/{cfg.gcp_project}"
            try:
                client.create_secret(
                    request={
                        "parent": project_path,
                        "secret_id": cfg.gcp_secret_id,
                        "secret": {"replication": {"automatic": {}}},
                    }
                )
                client.add_secret_version(
                    request={"parent": parent, "payload": {"data": payload}}
                )
            except Exception as create_err:
                print(
                    f"Error creating/adding version in GCP: {create_err}",
                    file=sys.stderr,
                )
                sys.exit(1)
        else:
            print(f"Error adding secret version in GCP: {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    app()
