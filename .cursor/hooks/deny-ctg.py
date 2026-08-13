#!/usr/bin/env python3
import json
import re
import sys

CTG_COMMAND = re.compile(
    r"(?:^|[;&|()\n\r])\s*(?:builtin\s+|command\s+|exec\s+|sudo\s+)?(?:[./\w-]*/)?ctgx?\b"
)


def deny(reason):
    print(
        json.dumps(
            {
                "permission": "deny",
                "userMessage": reason,
                "agentMessage": reason,
            }
        )
    )


def main():
    try:
        data = json.loads(sys.stdin.read() or "{}")
    except json.JSONDecodeError:
        return 0

    command = data.get("command", "")
    if command and CTG_COMMAND.search(command):
        deny(
            "AI agents are forbidden from executing ctg or ctgx commands in this workspace."
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
