#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "cyclopts>=4.5.1",
#     "pydantic>=2.13.4",
#     "onepassword-sdk>=0.1.1",
# ]
# ///


# cottage.toml
"""
[upstream.dev-1password]
envfile = "./1password/dev.env.cott.age"  # Should export OP_SERVICE_ACCOUNT_TOKEN.
vars = {
  OP_VAULT_ID = "your-vault-uuid-here",
  OP_ITEM_TITLE = "myapp-dev-secrets",  # Or specify OP_ITEM_ID instead
}
plugin = "./examples/plugins/cottage-plugin-onepassword.py"
"""

# myapp/dev.json.cott.toml
"""
[upstream.dev-1password]
pull = true
push = true
"""

import asyncio
import json
import os
import sys

from cyclopts import App
from onepassword import Client, ItemCategory, ItemField, ItemFieldType, ItemCreateParams
from pydantic import BaseModel, Field, model_validator


class OPSecretConfig(BaseModel):
    model_config = {"extra": "ignore"}
    op_service_account_token: str = Field(..., alias="OP_SERVICE_ACCOUNT_TOKEN")
    op_vault_id: str = Field(..., alias="OP_VAULT_ID")
    op_item_id: str | None = Field(None, alias="OP_ITEM_ID")
    op_item_title: str | None = Field(None, alias="OP_ITEM_TITLE")

    @model_validator(mode="after")
    def check_ids(self) -> "OPSecretConfig":
        if not self.op_item_id and not self.op_item_title:
            raise ValueError("Either OP_ITEM_ID or OP_ITEM_TITLE must be provided")
        return self

    def model_post_init(self, __context):
        print(  # Use --debug to see this message
            "Parsed configuration:", self, file=sys.stderr
        )


async def find_item_by_title(client: Client, vault_id: str, title: str):
    overviews = await client.items.list(vault_id)
    for ov in overviews:
        if ov.title == title:
            return await client.items.get(vault_id, ov.id)
    return None


async def pull_async(cfg: OPSecretConfig):
    client = await Client.authenticate(
        auth=cfg.op_service_account_token,
        integration_name="Cottage Integration",
        integration_version="1.0.0",
    )
    if cfg.op_item_id:
        item = await client.items.get(cfg.op_vault_id, cfg.op_item_id)
    else:
        item = await find_item_by_title(client, cfg.op_vault_id, cfg.op_item_title)
        if not item:
            print(
                f"Item with title '{cfg.op_item_title}' not found in vault '{cfg.op_vault_id}'",
                file=sys.stderr,
            )
            sys.exit(1)

    secrets = {}
    for field in item.fields:
        if field.title and field.value is not None:
            secrets[field.title] = field.value
    print(json.dumps(secrets))


async def push_async(cfg: OPSecretConfig, payload: dict):
    client = await Client.authenticate(
        auth=cfg.op_service_account_token,
        integration_name="Cottage Integration",
        integration_version="1.0.0",
    )

    fields = []
    for key, value in payload.items():
        fields.append(
            ItemField(
                id=key.lower().replace(" ", "_"),
                title=key,
                field_type=ItemFieldType.CONCEALED,
                value=str(value),
            )
        )

    item = None
    if cfg.op_item_id:
        try:
            item = await client.items.get(cfg.op_vault_id, cfg.op_item_id)
        except Exception:
            pass
    elif cfg.op_item_title:
        item = await find_item_by_title(client, cfg.op_vault_id, cfg.op_item_title)

    if item:
        item.fields = fields
        await client.items.put(item)
        print(f"Updated 1Password item '{item.title}'", file=sys.stderr)
    else:
        title = cfg.op_item_title or "Cottage Secrets"
        params = ItemCreateParams(
            title=title,
            category=ItemCategory.LOGIN,
            vault_id=cfg.op_vault_id,
            fields=fields,
        )
        new_item = await client.items.create(params)
        print(
            f"Created new 1Password item '{new_item.title}' (ID: {new_item.id})",
            file=sys.stderr,
        )


app = App()


@app.command()
def pull():
    cfg = OPSecretConfig.model_validate(os.environ)
    asyncio.run(pull_async(cfg))


@app.command()
def push():
    cfg = OPSecretConfig.model_validate(os.environ)
    payload = json.loads(input())
    asyncio.run(push_async(cfg, payload))


if __name__ == "__main__":
    app()
