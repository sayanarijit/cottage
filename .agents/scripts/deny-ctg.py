#!/usr/bin/env python3
import json
import os
import re
import sys

COTT_SUFFIX = re.compile(r"\.cott\.[^./\\]+$")
TOKEN_RE = re.compile(r"""[^\s"'`|;&<>()]+""")
PATH_KEY_HINT = re.compile(r"path|file|dir|target|absolute", re.IGNORECASE)


def repo_root():
    d = os.getcwd()
    while True:
        if os.path.isdir(os.path.join(d, ".git")) or os.path.isdir(os.path.join(d, ".cottage")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            return os.getcwd()
        d = parent


def is_sensitive(path, root):
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


def find_sensitive(value, root, path_context=False):
    """Only treat strings as path candidates when reached through a
    path/file/dir-hinted key, so file *content* being written (which may
    legitimately mention .cottage or *.cott.* in prose) is never mistaken
    for a path to protect."""
    if isinstance(value, str):
        return value if path_context and is_sensitive(value, root) else None
    if isinstance(value, dict):
        for key, nested in value.items():
            hit = find_sensitive(nested, root, bool(PATH_KEY_HINT.search(str(key))))
            if hit:
                return hit
    elif isinstance(value, list):
        for nested in value:
            hit = find_sensitive(nested, root, path_context)
            if hit:
                return hit
    return None


def allow():
    print(json.dumps({"decision": "allow"}))


def deny(reason):
    print(json.dumps({"decision": "deny", "reason": reason}))


def main():
    try:
        raw_input = sys.stdin.read()
        if not raw_input.strip():
            allow()
            return
        data = json.loads(raw_input)
        args = data.get("toolCall", {}).get("args", {})
        cmd = args.get("CommandLine", "")

        if re.search(r"(?:\b|/)ctg(?:x)?\b", cmd):
            deny("AI agents are forbidden from executing ctg or ctgx commands in this workspace.")
            return

        root = repo_root()
        hit = find_sensitive(args, root)
        if hit:
            deny(
                "Blocked by cottage workspace policy: '{}' is a protected secret "
                "(inside .cottage/, matches *.cott.*, or is a decrypted file with a "
                ".cott.age counterpart). AI agents must not view or edit it.".format(hit)
            )
            return

        allow()
    except Exception:
        allow()


if __name__ == "__main__":
    main()
