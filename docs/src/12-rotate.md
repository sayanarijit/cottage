# Rotating Recipients

Scenarios for re-encrypting secrets after the list of recipients changes:

1.  [I want to re-encrypt secrets for updated recipients without decrypting them to disk](#i-want-to-re-encrypt-secrets-for-updated-recipients-without-decrypting-them-to-disk)
2.  [I want to preview what rotation would change](#i-want-to-preview-what-rotation-would-change)
3.  [I want to automate secret rotation when recipient changes](#i-want-to-automate-secret-rotation-when-recipient-changes)

## I want to re-encrypt secrets for updated recipients without decrypting them to disk

After adding or removing a file in `.cottage/recipients`, existing secrets are still encrypted for the old recipients. Run `ctg rotate`. Like `ctg encrypt` and `ctg decrypt`, the path is optional and can be a file or a directory. It defaults to the project root:

```bash
ctg rotate                       # every secret in the project
ctg rotate ./secrets             # every secret under a directory
ctg rotate secret1.env.cott.age  # a single secret
```

Just like `ctg tee`, the cleartext is never written to disk. Each secret is decrypted in memory with your identity and immediately encrypted for the current recipients. The output looks like `ctg tee`, except that the input is the encrypted path instead of `-`:

> ```stdout
> encrypt secret1.env.cott.age
>    into secret1.env.cott.age
>    edit secret1.env.cott.toml
> ```

The recipients checksum is not verified, since it is expected to change. The checksum of the encrypted file is still verified; pass `--force` to skip that too. Any `allow`/`deny` rules in a secret's metadata keep applying.

You need to be a recipient of a secret to rotate it, as your identity is used to decrypt it. Plain and metadata paths are accepted as well, and resolve to the matching encrypted file.

## I want to preview what rotation would change

Pass `-n` to do a dry run. Nothing is written:

```bash
ctg rotate -n
```

## I want to automate secret rotation when recipient changes

If you use GitHub Actions, you can use add he following workflow to automatically rotate secrets when the recipients change:

```yaml
# .github/workflows/cottage-rotate.yml
name: Cottage Rotate
on:
  push:
    paths:
      - ".cottage/recipients/**"
    branches:
      - main
permissions:
  contents: write
jobs:
  verify-secrets:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - name: Rotate secrets
        env:
          COTTAGE_IDENTITY: ${{ secrets.COTTAGE_IDENTITY }}
        run: docker run --rm -v "${{ github.workspace }}:/app" -e COTTAGE_IDENTITY ghcr.io/sayanarijit/cottage rotate
      - name: Commit and push
        run: |
          git config --global user.name "${{ github.actor }}"
          git config --global user.email "${{ github.actor }}@users.noreply.github.com"
          if ! git commit -am "cottage: auto rotate secrets"; then
            echo "No changes to commit"
            exit 0
          fi
          git push origin HEAD:main
```

This requires that the `COTTAGE_IDENTITY` environment variable is set to a valid identity in the repository.
