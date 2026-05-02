Set Shell "bash"
Set FontSize 22
Set Width 1200
Set Height 900
Set Padding 40
Set Theme "Chalk"
Set WindowBar Colorful
Set TypingSpeed 50ms

# Output
Output demo.gif

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

# Step 3: Temporary run (the killer feature)
Type "# 4. Run a command with temporary decrypted secrets" Enter
Sleep 500ms
Type "ctgx grep API_KEY secret.env" Enter
Sleep 3s

# Verify it was cleaned up
Type "# Decrypted files are deleted immediately after execution" Enter
Sleep 500ms
Type "ls secret.env" Enter
Sleep 2s

# Step 4: Persistent decryption
Type "# 5. Decrypt secrets for persistent local development" Enter
Sleep 500ms
Type "ctg decrypt" Enter
Sleep 1s
Type "ls -F" Enter
Sleep 2s

# Step 5: Status, Diff and Sync
Type "# 6. Modify the secret and see the diff" Enter
Sleep 500ms
Type "echo 'NEW_VAR=value' >> secret.env" Enter
Sleep 500ms
Type "ctg status" Enter
Sleep 500ms
Type "ctg diff" Enter
Sleep 3s

Type "# Sync back the changes to the encrypted file" Enter
Sleep 500ms
Type "ctg sync" Enter
Sleep 2s

# Step 6: Clean up
Type "# 7. Clean up all decrypted secrets from the repo" Enter
Sleep 500ms
Type "ctg clean" Enter
Sleep 1s
Type "ls -F" Enter
Sleep 2s

# Finish
Type "# Done! Cottage: A modern git based age-encrypted secrets manager for teams" Enter
Sleep 4s
