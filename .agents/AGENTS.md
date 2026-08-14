# Workspace Instructions for AI agents

- **Command Safety**: AI agents are strictly forbidden from running `ctg` or `ctgx` commands under any circumstances.
- **Secret Files**: AI agents must never view, read, edit, create, or otherwise access:
  - Anything inside a `.cottage/` directory (identities, recipients, project state).
  - Any file matching `*.cott.*` (e.g. `*.cott.age` encrypted blobs, `*.cott.toml` redacted previews).
  - Any decrypted file `{file}` that has a corresponding `{file}.cott.age` encrypted counterpart on disk.
