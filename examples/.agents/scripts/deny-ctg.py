#!/usr/bin/env python3
import json
import re
import sys


def main():
    try:
        raw_input = sys.stdin.read()
        if not raw_input.strip():
            print(json.dumps({"decision": "allow"}))
            return
        data = json.loads(raw_input)
        cmd = data.get("toolCall", {}).get("args", {}).get("CommandLine", "")
        if re.search(r"(?:\b|/)ctg(?:x)?\b", cmd):
            print(
                json.dumps(
                    {
                        "decision": "deny",
                        "reason": "AI agents are forbidden from executing ctg or ctgx commands in this workspace.",
                    }
                )
            )
        else:
            print(json.dumps({"decision": "allow"}))
    except Exception:
        print(json.dumps({"decision": "allow"}))


if __name__ == "__main__":
    main()
