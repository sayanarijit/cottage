#!/usr/bin/env python3
import json
import re
import sys

COMMAND_KEYS = {"cmd", "command", "Command", "CommandLine", "commandLine"}
CTG_COMMAND = re.compile(
    r"(?:^|[;&|()\n\r])\s*(?:builtin\s+|command\s+|exec\s+|sudo\s+)?(?:[./\w-]*/)?ctgx?\b"
)


def find_command(value):
    if isinstance(value, dict):
        for key, nested in value.items():
            if key in COMMAND_KEYS and isinstance(nested, str):
                return nested
        for nested in value.values():
            command = find_command(nested)
            if command:
                return command
    elif isinstance(value, list):
        for nested in value:
            command = find_command(nested)
            if command:
                return command
    return ""


def deny(reason):
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            }
        )
    )


def main():
    try:
        data = json.loads(sys.stdin.read() or "{}")
    except json.JSONDecodeError:
        return 0

    command = find_command(data)
    if command and CTG_COMMAND.search(command):
        deny(
            "AI agents are forbidden from executing ctg or ctgx commands in this workspace."
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
