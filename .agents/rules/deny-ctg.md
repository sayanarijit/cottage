# AGY Rule: Deny `ctg` Command Execution and Secret File Access

The `ctg` and `ctgx` CLI commands are strictly forbidden from being executed by AGY (the agent) in this workspace.

- Never propose or run any `ctg` or `ctgx` commands (e.g., `ctg clean`, `ctg decrypt`, `ctg run`, `ctg env`, etc.).
- Never invoke lifecycle hooks or background tasks that execute `ctg` or `ctgx`.

AGY must also never view, read, edit, create, or otherwise access secret files:

- Anything inside a `.cottage/` directory.
- Any file matching `*.cott.*` (encrypted `*.cott.age` blobs and redacted `*.cott.toml` previews).
- Any decrypted file `{file}` that has a corresponding `{file}.cott.age` encrypted counterpart on disk.
