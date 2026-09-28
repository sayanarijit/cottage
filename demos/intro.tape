Set Shell "bash"
Set FontSize 22
Set Width 1200
Set Height 900
Set Padding 40
Set Theme "Chalk"
Set WindowBar Colorful
Set TypingSpeed 50ms

# Output
Output intro.gif

# Initialize cottage
Type "# Initialize cottage in a git repo" Enter
Sleep 500ms
Type "mkdir demo && cd demo" Enter
Sleep 1s
Type "git init" Enter
Sleep 500ms
Type "ctg init" Enter Enter
Sleep 2s

# Show generated files and keys
Type "ls -aF" Enter
Sleep 500ms
Type "cat .cottage/recipients/$USER" Enter Enter Enter
Sleep 1s
Type "cat ~/.config/cottage/identity/demo/*.key" Enter Enter
Sleep 2s

# Create and edit a secret
Type "# Create and edit a secret (supports stdin or $EDITOR)" Enter
Sleep 500ms
Type "ctg edit secret.env <<EOF" Enter
Sleep 500ms
Type "API_KEY=super-secret-key" Enter
Sleep 500ms
Type "DATABASE_URL=postgres://user:password@localhost/db" Enter
Sleep 500ms
Type "EOF" Enter
Sleep 2s

# Show the encrypted results
Type "# The secret is now encrypted and git-ignored" Enter
Sleep 500ms
Type "ls -F" Enter
Sleep 2s
Type "# Redacted preview is stored in metadata for visibility" Enter
Sleep 500ms
Type "cat secret.env.cott.toml" Enter
Sleep 3s

# Verify secrets
Type "# Verify encrypted secrets match metadata" Enter
Sleep 500ms
Type "ctg verify" Enter
Sleep 2s

# Temporary run (the killer feature)
Type "# Clean repo first so secrets are absent before running ctgx" Enter
Sleep 500ms
Type "ctg clean" Enter
Sleep 1s

Type "# Run a command with temporary decrypted secrets" Enter
Sleep 500ms
Type "ctgx grep API_KEY secret.env" Enter
Sleep 3s

# Verify it was cleaned up
Type "# Decrypted secrets are only deleted automatically if absent before" Enter
Sleep 500ms
Type "ls secret.env" Enter
Sleep 2s

# Environment injection workflow
Type "# Inject secrets directly as environment variables" Enter
Sleep 500ms
Type "ctg env -F secret.env.cott.age -- printenv API_KEY" Enter
Sleep 3s

# Persistent decryption
Type "# Decrypt secrets for persistent local development" Enter
Sleep 500ms
Type "ctg decrypt" Enter
Sleep 1s
Type "ls -F" Enter
Sleep 2s

# Status, Diff and Sync
Type "# Modify secret and see status, flags & diff" Enter
Sleep 500ms
Type "echo 'NEW_VAR=value' >> secret.env" Enter
Sleep 500ms
Type "ctg status" Enter
Sleep 500ms
Type "ctg status --skip-encryption" Enter
Sleep 500ms
Type "ctg diff" Enter
Sleep 3s

Type "# Sync back changes to the encrypted file" Enter
Sleep 500ms
Type "ctg sync" Enter
Sleep 2s

# Clean up
Type "# Clean up decrypted secrets" Enter
Sleep 500ms
Type "ctg clean" Enter
Sleep 1s
Type "ls -F" Enter
Sleep 2s

# Finish
Type "# Done! cottage: A modern git-based age-encrypted secrets manager for teams" Enter
Sleep 4s
