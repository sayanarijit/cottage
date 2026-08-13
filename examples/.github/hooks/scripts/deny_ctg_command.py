#!/usr/bin/env python3

import json
import re
import sys


TERMINAL_TOOL_NAMES = {
    "bash",
    "run_in_terminal",
    "runinterminal",
    "terminal",
    "shell",
}


def _normalize_tool_name(value):
    return re.sub(r"[^a-z]", "", str(value or "").lower())


def _extract_command(tool_input):
    if not isinstance(tool_input, dict):
        return ""

    for key in ("command", "cmd", "text"):
        value = tool_input.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()

    args = tool_input.get("args")
    if isinstance(args, list):
        parts = [part for part in args if isinstance(part, str) and part.strip()]
        if parts:
            return " ".join(parts)

    return ""


def main():
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        return 0

    tool_name = _normalize_tool_name(payload.get("tool_name"))
    if tool_name not in TERMINAL_TOOL_NAMES:
        return 0

    command = _extract_command(payload.get("tool_input"))
    if not re.match(r"^ctg(?:\s|$)", command):
        return 0

    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": "Direct ctg shell commands are blocked by workspace policy; rely on the session hooks instead.",
                "additionalContext": "A workspace hook already runs 'ctg clean -qqq' at session start and on every prompt submission."
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())