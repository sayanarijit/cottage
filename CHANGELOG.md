# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.1](https://github.com/sayanarijit/cottage/compare/v0.7.0...v0.7.1) - 2026-08-27

### Added

- *(gitignore)* faster scan by respecting .ignore files

## [0.7.0](https://github.com/sayanarijit/cottage/compare/v0.6.8...v0.7.0) - 2026-08-24

### Added

- *(run --clean)* add ctg run --clean
- *(plugin)* add cottage plugin for github secrets ([#83](https://github.com/sayanarijit/cottage/pull/83))

### Fixed

- *(docs)* [**breaking**] update docs to clearify the clean behaviour
- *(--clean)* --clean should always cleanup decrypted files

### Other

- *(example)* update example
- *(ceps)* update deps
- *(readme)* include github secrets plugin in readme

## [0.6.8](https://github.com/sayanarijit/cottage/compare/v0.6.7...v0.6.8) - 2026-08-15

### Added

- *(ai-safety)* AI agents should ignore sensitive files

### Fixed

- *(doc)* update re-encrypt doc
- *(gitattribute)* export-ignore -> linguist-generated ([#82](https://github.com/sayanarijit/cottage/pull/82))

## [0.6.7](https://github.com/sayanarijit/cottage/compare/v0.6.6...v0.6.7) - 2026-08-14

### Fixed

- *(publish)* fix pypi

## [0.6.6](https://github.com/sayanarijit/cottage/compare/v0.6.5...v0.6.6) - 2026-08-14

### Added

- *(ai)* add cursor and copilot integration
- *(ai)* ai session end shouldn't auto decrypt secrets
- *(agy)* add antigravity integration
- *(claude)* adds claude code hooks and permission examples

### Fixed

- *(agy)* fix agy hook script
- *(ai)* cleanup secrets at turn start
- *(claude)* allow ctg hooks to fail

### Other

- *(deps)* update dependencies
- add vscode plugin docs
- fix outdated demo
- *(deps)* bump clap_complete from 4.6.5 to 4.6.7
- *(deps)* bump clap from 4.6.1 to 4.6.2
- *(deps)* bump anyhow from 1.0.103 to 1.0.104
- *(deps)* bump toml from 1.1.2+spec-1.1.0 to 1.1.3+spec-1.1.0
- *(deps)* bump serde from 1.0.228 to 1.0.229
- add npm version in readme

## [0.6.5](https://github.com/sayanarijit/cottage/compare/v0.6.4...v0.6.5) - 2026-06-27

### Other

- *(fix npm)* fix publishing npm package

## [0.6.4](https://github.com/sayanarijit/cottage/compare/v0.6.3...v0.6.4) - 2026-06-27

### Added

- *(npm)* add npm package

### Fixed

- *(age-identity)* multiline age identity files with comments
- *(ci)* fix npm publish CI

### Other

- *(tests)* add more tests
- *(deps)* Update deps and add tests

## [0.6.3](https://github.com/sayanarijit/cottage/compare/v0.6.2...v0.6.3) - 2026-06-10

### Fixed

- pull/push from subdirectory

### Other

- *(deps)* bump log from 0.4.29 to 0.4.30
- *(deps)* bump assert_fs from 1.1.3 to 1.1.4
- cleanup redundant docs
- list the plugins in readme and docs
- add more plugin examples (mostly AI generated)
- Add direct vault plugin example

## [0.6.2](https://github.com/sayanarijit/cottage/compare/v0.6.1...v0.6.2) - 2026-06-06

### Added

- feat!(upstream): Check status pre-requirements decryption

### Fixed

- *(upstream)* Update timestamps post-requirements-decryption

### Other

- format
- Simplify plugin features description in documentation
- Improve plugin docs
- Rearrange KUBE_NAMESPACE and VAULT_MOUNT in config
- Update required secrets file in cottage.toml
- Rename upstream from 'myvault' to 'customvault'
- Fix examples doc
- minor docs cleanup
- *(plugin)* Update cottage.toml example docs

## [0.6.1](https://github.com/sayanarijit/cottage/compare/v0.6.0...v0.6.1) - 2026-06-06

### Added

- *(upstream)* Auto require decrypted secrets from vars
- *(upstream)* Declare upstream required secrets

### Fixed

- *(plugin)* Fix plugin execution
- *(clean)* Cleanup temporary decrypted files reliably

### Other

- *(plugin)* Update plugin to use `requires` attr
- *(config)* Simplify configuration inheritence
- *(config)* Add more tests for config inheritence
- Another minor example cleanup
- Minor cleanup of example
- *(plugin)* Add working plugin that also serves as example

## [0.6.0](https://github.com/sayanarijit/cottage/compare/v0.5.6...v0.6.0) - 2026-06-05

### Added

- *(cli)* [**breaking**] ctg keygen

### Other

- *(deps)* bump similar from 3.1.0 to 3.1.1

## [0.5.6](https://github.com/sayanarijit/cottage/compare/v0.5.5...v0.5.6) - 2026-05-27

### Other

- Securely remove file
- *(deps)* bump clap_complete from 4.6.4 to 4.6.5
- *(deps)* bump filetime from 0.2.28 to 0.2.29
- *(sync)* add link to cottage sync
- *(doc)* remove cottage link

## [0.5.5](https://github.com/sayanarijit/cottage/compare/v0.5.4...v0.5.5) - 2026-05-12

### Other

- fix typos

## [0.5.4](https://github.com/sayanarijit/cottage/compare/v0.5.3...v0.5.4) - 2026-05-12

### Fixed

- *(precommit)* fix post rewrite checks

### Other

- *(post-pull)* finish the scenario

## [0.5.3](https://github.com/sayanarijit/cottage/compare/v0.5.2...v0.5.3) - 2026-05-12

### Fixed

- *(hooks)* fix git hooks on pull

### Other

- *(book)* add custom providers and plugins
- *(book)* improve access docs in the book
- *(book)* document access and git hook
- *(githooks)* document scenarios for git hooks
- *(book)* add Collaboration
- *(mdbook)* improve some docs

## [0.5.2](https://github.com/sayanarijit/cottage/compare/v0.5.1...v0.5.2) - 2026-05-12

### Fixed

- *(dryrun)* handle dry-run properly

### Other

- TOC for the config specs
- improve examples
- Fix broken links
- add recipient and decryption scenarios
- Cottage -> cottage
- add more scenarios
- add more scenarios around init and keys
- fix command

## [0.5.1](https://github.com/sayanarijit/cottage/compare/v0.5.0...v0.5.1) - 2026-05-11

### Fixed

- *(windows)* fix compile on windows
- *(perm)* chmod 600 the identity key in unix

### Other

- *(link)* fix link in doc
- *(links)* use - instead of _

## [0.5.0](https://github.com/sayanarijit/cottage/compare/v0.4.5...v0.5.0) - 2026-05-10

### Added

- *(i686)* add target i686-unknown-linux-musl

### Fixed

- *(push/pull)* [**breaking**] use the encrypted secret paths, not metadata path

### Other

- Potential fix for code scanning alert no. 1: Workflow does not contain permissions
- *(deps)* bump clap_complete from 4.6.3 to 4.6.4 ([#37](https://github.com/sayanarijit/cottage/pull/37))
- *(deps)* bump filetime from 0.2.27 to 0.2.28 ([#36](https://github.com/sayanarijit/cottage/pull/36))
- Set package ecosystem to 'cargo' in dependabot config
- Update SECURITY.md to reflect new reporting process
- *(cottagelab)* add lab link to try cottage without installing
- *(readme)* fix typo
- *(readme)* add links to similar projects
- *(spec)* document toml specs.

## [0.4.5](https://github.com/sayanarijit/cottage/compare/v0.4.4...v0.4.5) - 2026-05-08

### Fixed

- *(cwd)* fix pull/push cwd behavior

## [0.4.4](https://github.com/sayanarijit/cottage/compare/v0.4.3...v0.4.4) - 2026-05-08

### Added

- *(pull/push)* add `ctg pull` and `ctg push`

### Fixed

- *(tests)* fix rust tests

### Other

- *(typos)* fix some docs

## [0.4.3](https://github.com/sayanarijit/cottage/compare/v0.4.2...v0.4.3) - 2026-05-07

### Added

- *(env)* add `ctg env` command to export dotenv
- *(file permission)* chmod 0600 decrypted files

### Fixed

- *(run)* improve `ctg run`/`ctgx` command help
- *(output)* improve how cottage prints output

### Other

- *(env)* Document `ctg env` usage.

## [0.4.2](https://github.com/sayanarijit/cottage/compare/v0.4.1...v0.4.2) - 2026-05-06

### Added

- *(clean all)* add option to undo all changes

## [0.4.1](https://github.com/sayanarijit/cottage/compare/v0.4.0...v0.4.1) - 2026-05-06

### Added

- *(docker)* release docker images
- *(docker)* add docker image

### Fixed

- *(ci)* fix CI ctg verify
- *(docker)* fix docker image and docs

### Other

- *(ci)* Fix CI verify docs
- *(readme)* fix gh action example
- *(badge)* fix readme badge
- *(action)* update gh action permission with docs

## [0.4.0](https://github.com/sayanarijit/cottage/compare/v0.3.2...v0.4.0) - 2026-05-06

### Added

- *(force)* restore --force for encryption

### Other

- *(force-encrypt)* [**breaking**] encryption no longer requires verification

## [0.3.2](https://github.com/sayanarijit/cottage/compare/v0.3.1...v0.3.2) - 2026-05-06

### Fixed

- *(precommit)* don't pass filenames on decrypt

### Other

- *(verify)* document veriify usage
- *(verify)* verify checksum without decrypting secrets

## [0.3.1](https://github.com/sayanarijit/cottage/compare/v0.3.0...v0.3.1) - 2026-05-05

### Fixed

- *(pre-commit-hooks)* match only .cott.(age|toml)

## [0.3.0](https://github.com/sayanarijit/cottage/compare/v0.2.3...v0.3.0) - 2026-05-05

### Added

- feat(code) remove support for passphrase
- *(acl)* [**breaking**] add basic access control

### Fixed

- *(hash)* [**breaking**] match recipient checksum using only the valid part
- *(enc)* [**breaking**] verify intended recipients match metadata

### Other

- *(diff)* fix diff tests
- *(code)* minor cleanups
- *(deps)* remove rpassword
- *(acl)* Document the access control feature in readme

## [0.2.3](https://github.com/sayanarijit/cottage/compare/v0.2.2...v0.2.3) - 2026-05-05

This is to be considered the initial release of the project, and is not expected to be stable. The API may change without a major version bump.

Previous releases had a security flaw where it stored the checksum of plain text secrets in the metadata file. While it's difficult, attackers could potentially use this to brute-force the secrets. And hence, the previous releases have been yanked.

This release removes the checksum from the metadata file.

If you are upgrading from a previous version, you will need to force re-encrypt (`ctg decrypt --force && ctg encrypt --force`) your secrets with this version to remove the checksum from the metadata file.
