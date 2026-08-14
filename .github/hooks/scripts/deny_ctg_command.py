#!/usr/bin/env python3

import json
import os
import re
import sys


TERMINAL_TOOL_NAMES = {
    "bash",
    "run_in_terminal",
    "runinterminal",
    "terminal",
    "shell",
}

COTT_SUFFIX = re.compile(r"\.cott\.[^./\\]+$")
TOKEN_RE = re.compile(r"""[^\s"'`|;&<>()]+""")
PATH_KEY_HINT = re.compile(r"path|file|dir|target|absolute", re.IGNORECASE)


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


def _repo_root(hint=None):
    d = os.path.abspath(hint) if hint and os.path.isdir(hint) else os.getcwd()
    while True:
        if os.path.isdir(os.path.join(d, ".git")) or os.path.isdir(os.path.join(d, ".cottage")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            return os.path.abspath(hint or os.getcwd())
        d = parent


def _is_sensitive(path, root):
    if not isinstance(path, str):
        return False
    path = path.strip().strip("'\"")
    if not path or len(path) > 4096 or "\n" in path:
        return False
    normalized = path.replace("\\", "/")
    if ".cottage" in [p for p in normalized.split("/") if p]:
        return True
    if COTT_SUFFIX.search(normalized):
        return True
    abs_path = path if os.path.isabs(path) else os.path.join(root, path)
    try:
        return os.path.exists(abs_path + ".cott.age")
    except OSError:
        return False


def _find_sensitive_in_command(command, root):
    if not command:
        return None
    for token in TOKEN_RE.findall(command):
        if _is_sensitive(token, root):
            return token
    return None


def _find_sensitive(value, root, path_context=False):
    """Only treat strings as path candidates when reached through a
    path/file/dir-hinted key (e.g. readFile/createFile/editFiles path
    arguments), so file content that merely mentions .cottage or
    *.cott.* in prose is never mistaken for a path to protect."""
    if isinstance(value, str):
        return value if path_context and _is_sensitive(value, root) else None
    if isinstance(value, dict):
        for key, nested in value.items():
            hit = _find_sensitive(nested, root, bool(PATH_KEY_HINT.search(str(key))))
            if hit:
                return hit
    elif isinstance(value, list):
        for nested in value:
            hit = _find_sensitive(nested, root, path_context)
            if hit:
                return hit
    return None


def _deny(reason, additional_context=None):
    output = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }
    if additional_context:
        output["hookSpecificOutput"]["additionalContext"] = additional_context
    json.dump(output, sys.stdout)
    sys.stdout.write("\n")


SECRET_REASON = (
    "Blocked by cottage workspace policy: '{}' is a protected secret "
    "(inside .cottage/, matches *.cott.*, or is a decrypted file with a "
    ".cott.age counterpart). AI agents must not view or edit it."
)


def main():
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        return 0

    tool_name = _normalize_tool_name(payload.get("tool_name"))
    tool_input = payload.get("tool_input")
    root = _repo_root(payload.get("cwd"))

    if tool_name in TERMINAL_TOOL_NAMES:
        command = _extract_command(tool_input)

        if re.match(r"^ctg(?:\s|$)", command):
            _deny(
                "Direct ctg shell commands are blocked by workspace policy; "
                "rely on the session hooks instead.",
                additional_context=(
                    "A workspace hook already runs 'ctg clean -qqq' at session "
                    "start and on every prompt submission."
                ),
            )
            return 0

        hit = _find_sensitive_in_command(command, root)
        if hit:
            _deny(SECRET_REASON.format(hit))
            return 0

    hit = _find_sensitive(tool_input, root)
    if hit:
        _deny(SECRET_REASON.format(hit))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
