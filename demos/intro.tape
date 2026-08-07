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

# Step 1: Initialize cottage
Type "# 1. Initialize cottage in a git repo" Enter
Sleep 500ms
Type "mkdir demo && cd demo" Enter
Sleep 1s
Type "git init" Enter
Sleep 500ms
Type "ctg init" Enter
Sleep 2s

# Show generated files
Type "ls -aF" Enter
Sleep 1s
Type "ls -F .cottage/" Enter
Sleep 2s

# Step 2: Create and edit a secret
Type "# 2. Create and edit a secret (supports stdin or $EDITOR)" Enter
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
Sleep 1s
Type "# Redacted preview is stored in metadata for visibility" Enter
Sleep 500ms
Type "cat secret.env.cott.toml" Enter
Sleep 3s

# Step 3: Verify secrets
Type "# 3. Verify encrypted secrets match metadata" Enter
Sleep 500ms
Type "ctg verify" Enter
Sleep 2s

# Step 4: Temporary run (the killer feature)
Type "# 4. Clean repo first so secrets are absent before running ctgx" Enter
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

# Step 5: Environment injection workflow
Type "# 5. Inject secrets directly as environment variables" Enter
Sleep 500ms
Type "ctg env -F secret.env.cott.age -- printenv API_KEY" Enter
Sleep 3s

# Step 6: Persistent decryption & Dry Run
Type "# 6. Preview & decrypt secrets for persistent local development" Enter
Sleep 500ms
Type "ctg decrypt --dry-run" Enter
Sleep 1s
Type "ctg decrypt" Enter
Sleep 1s
Type "ls -F" Enter
Sleep 2s

# Step 7: Status, Diff and Sync
Type "# 7. Modify secret and see status, flags & diff" Enter
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

# Step 8: Shell Completions
Type "# 8. Generate shell completions for your shell" Enter
Sleep 500ms
Type "ctg autocomplete bash | head -n 10" Enter
Sleep 2s

# Step 9: Clean up (Dry-run & actual)
Type "# 9. Preview clean with --dry-run and clean up decrypted secrets" Enter
Sleep 500ms
Type "ctg clean --dry-run" Enter
Sleep 1s
Type "ctg clean" Enter
Sleep 1s
Type "ls -F" Enter
Sleep 2s

# Finish
Type "# Done! cottage: A modern git-based age-encrypted secrets manager for teams" Enter
Sleep 4s
