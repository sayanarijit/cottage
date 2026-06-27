#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "boto3>=1.34.0",
# ]
# ///


# cottage.toml
"""
[upstream.dev-aws]
envfile = "./aws/dev.env.cott.age"  # Export AWS credentials or configure environment.
vars = {
  AWS_SECRET_ID = "myapp/dev/secrets",
  AWS_REGION_NAME = "us-east-1",
}
plugin = "./examples/plugins/cottage-plugin-aws-secretsmanager.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-aws]
pull = true
push = true
"""

import json
import os
import sys

import boto3
from cyclopts import App
from pydantic import BaseModel, Field


class AWSSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    aws_secret_id: str = Field(..., alias="AWS_SECRET_ID")
    aws_region_name: str | None = Field(None, alias="AWS_REGION_NAME")
    aws_access_key_id: str | None = Field(None, alias="AWS_ACCESS_KEY_ID")
    aws_secret_access_key: str | None = Field(None, alias="AWS_SECRET_ACCESS_KEY")
    aws_session_token: str | None = Field(None, alias="AWS_SESSION_TOKEN")
    aws_profile: str | None = Field(None, alias="AWS_PROFILE")

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


def get_aws_client(config: AWSSecretConfig):
    session = boto3.session.Session(
        aws_access_key_id=config.aws_access_key_id,
        aws_secret_access_key=config.aws_secret_access_key,
        aws_session_token=config.aws_session_token,
        profile_name=config.aws_profile,
        region_name=config.aws_region_name,
    )
    return session.client("secretsmanager")


app = App()


@app.command()
def pull():
    cfg = AWSSecretConfig.model_validate(os.environ)
    client = get_aws_client(cfg)
    print(  # Use --debug to see this message
        f"Pulling secret '{cfg.aws_secret_id}' from AWS Secrets Manager...",
        file=sys.stderr,
    )
    try:
        resp = client.get_secret_value(SecretId=cfg.aws_secret_id)
    except Exception as e:
        print(f"Error retrieving secret from AWS: {e}", file=sys.stderr)
        sys.exit(1)

    if "SecretString" in resp:
        secret_str = resp["SecretString"]
        try:
            # Try to return as JSON if it's a JSON object
            data = json.loads(secret_str)
            print(json.dumps(data))
        except json.JSONDecodeError:
            # Fallback to returning the raw string wrapped in a dict
            print(json.dumps({"value": secret_str}))
    elif "SecretBinary" in resp:
        binary_data = resp["SecretBinary"]
        print(json.dumps({"value": binary_data.decode("utf-8")}))
    else:
        print("No secret data found in response", file=sys.stderr)
        sys.exit(1)


@app.command()
def push():
    cfg = AWSSecretConfig.model_validate(os.environ)
    client = get_aws_client(cfg)
    payload_str = json.dumps(json.loads(input()))
    print(  # Use --debug to see this message
        f"Pushing secret '{cfg.aws_secret_id}' to AWS Secrets Manager...",
        file=sys.stderr,
    )
    try:
        client.put_secret_value(SecretId=cfg.aws_secret_id, SecretString=payload_str)
    except client.exceptions.ResourceNotFoundException:
        print(
            f"Secret '{cfg.aws_secret_id}' not found. Creating it...",
            file=sys.stderr,
        )
        try:
            client.create_secret(Name=cfg.aws_secret_id, SecretString=payload_str)
        except Exception as e:
            print(f"Error creating secret in AWS: {e}", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        print(f"Error updating secret in AWS: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    app()
