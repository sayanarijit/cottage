# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.1](https://github.com/sayanarijit/cottage/compare/v0.5.0...v0.5.1) - 2026-05-11

### Fixed

- *(windows)* fix compile on windows
- *(perm)* chmod 600 the identity key in unix

### Other

- *(toc)* add toc in each chapter
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
