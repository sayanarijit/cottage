#!/usr/bin/env python3
import json
import os
import re
import sys

COTT_SUFFIX = re.compile(r"\.cott\.[^./\\]+$")


def repo_root(hint=None):
    d = os.path.abspath(hint) if hint and os.path.isdir(hint) else os.getcwd()
    while True:
        if os.path.isdir(os.path.join(d, ".git")) or os.path.isdir(os.path.join(d, ".cottage")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            return os.path.abspath(hint or os.getcwd())
        d = parent


def is_sensitive(path, root):
    if not isinstance(path, str):
        return False
    path = path.strip().strip("'\"")
    if not path or len(path) > 4096:
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


def deny(reason):
    print(
        json.dumps(
            {
                "permission": "deny",
                "userMessage": reason,
                "user_message": reason,
                "agentMessage": reason,
            }
        )
    )


def main():
    try:
        data = json.loads(sys.stdin.read() or "{}")
    except json.JSONDecodeError:
        return 0

    file_path = data.get("file_path", "")
    root = repo_root(data.get("cwd"))
    if file_path and is_sensitive(file_path, root):
        deny(
            "Blocked by cottage workspace policy: '{}' is a protected secret "
            "(inside .cottage/, matches *.cott.*, or is a decrypted file with a "
            ".cott.age counterpart). AI agents must not view or edit it.".format(file_path)
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
