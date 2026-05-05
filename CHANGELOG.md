# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
