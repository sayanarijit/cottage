# cottage Configuration Specification

This document describes the specification of `cottage.toml` and `*.cott.toml` files used by cottage.

1.  [Project Configuration - cottage.toml](#project-configuration---cottage.toml)
    1. [Root Fields](#root-fields)
    2. [UpstreamConfig](#upstreamconfig)
    3. [PullPushConfig](#pullpushconfig)
2.  [Secret Metadata - .cott.toml](#secret-metadata---.cott.toml)
    1. [Root Fields](#root-fields)
    2. [ChecksumMetadata](#checksummetadata)
    3. [PreviewMetadata](#previewmetadata)
    4. [SecretMetadata](#secretmetadata)
    5. [UpstreamMetadata](#upstreammetadata)

## Project Configuration - cottage.toml

The `cottage.toml` file is located at the project root and defines global and upstream settings.

### Root Fields

| Field      | Type                                           | Description                                                            |
| ---------- | ---------------------------------------------- | ---------------------------------------------------------------------- |
| `upstream` | Map<String, [UpstreamConfig](#upstreamconfig)> | Optional. Defines upstream configurations for pulling/pushing secrets. |

### UpstreamConfig

These settings can be defined at the top level of an upstream or within its `pull`/`push` sections.

| Field      | Type                              | Description                                                                                                              |
| ---------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `cwd`      | Boolean                           | Optional. If true, run the script in the directory of the secret.                                                        |
| `envfile`  | Path                              | Optional. Path to an encrypted file to use as environment variables for the script.                                      |
| `vars`     | Map<String, String>               | Optional. Environment variables to pass to the script. If any variable value is a path to a decrypted secret that has a corresponding encrypted counterpart, it is automatically added to the `requires` list. |
| `requires` | Array<Path>                       | Optional. List of secret paths to be decrypted before running push/pull operations (and securely cleaned up afterwards). |
| `shell`    | String                            | Optional. The shell to use for running scripts (default: `sh`).                                                          |
| `pull`     | [PullPushConfig](#pullpushconfig) | Optional. Specific configuration for the pull operation.                                                                 |
| `push`     | [PullPushConfig](#pullpushconfig) | Optional. Specific configuration for the push operation.                                                                 |
| `plugin`   | String                            | Optional. Path to a plugin executable.                                                                                   |

### PullPushConfig

Inherits defaults from `UpstreamConfig`.

| Field      | Type                | Description                                              |
| ---------- | ------------------- | -------------------------------------------------------- |
| `cwd`      | Boolean             | Optional.                                                |
| `envfile`  | Path                | Optional.                                                |
| `vars`     | Map<String, String> | Optional.                                                |
| `requires` | Array<Path>         | Optional.                                                |
| `shell`    | String              | Optional.                                                |
| `script`   | String              | Optional. The shell script to execute for the operation. |
| `plugin`   | String              | Optional. Path to a plugin executable.                   |

---

## Secret Metadata - .cott.toml

Every encrypted file `*.cott.age` has a corresponding `*.cott.toml` metadata file.

### Root Fields

| Field      | Type                                               | Description                                                                     |
| ---------- | -------------------------------------------------- | ------------------------------------------------------------------------------- |
| `checksum` | [ChecksumMetadata](#checksummetadata)              | Auto generated. Integrity checks for the encrypted data and recipients.         |
| `preview`  | [PreviewMetadata](#previewmetadata)                | Auto generated for specific file types. Values-redacted preview of the content. |
| `secret`   | [SecretMetadata](#secretmetadata)                  | Metadata about the secret itself.                                               |
| `upstream` | Map<String, [UpstreamMetadata](#upstreammetadata)> | Optional. Upstream-specific settings for this secret.                           |

### ChecksumMetadata

| Field        | Type   | Description                                                              |
| ------------ | ------ | ------------------------------------------------------------------------ |
| `encrypted`  | String | BLAKE3 checksum of the encrypted file content (prefixed with `blake3:`). |
| `recipients` | String | BLAKE3 checksum of the recipients used to encrypt the file.              |

### PreviewMetadata

| Field     | Type   | Description                                             |
| --------- | ------ | ------------------------------------------------------- |
| `format`  | String | One of: `yaml`, `json`, `toml`, `dotenv`, `ini`, `hcl`. |
| `preview` | String | The value-redacted preview content.                     |

### SecretMetadata

| Field       | Type          | Description                                             |
| ----------- | ------------- | ------------------------------------------------------- |
| `timestamp` | String        | Auto generated. Last modified timestamp of the secret.  |
| `allow`     | Array<String> | Optional. List of glob patterns for allowed recipients. |
| `deny`      | Array<String> | Optional. List of glob patterns for denied recipients.  |

### UpstreamMetadata

| Field  | Type                | Description                                                              |
| ------ | ------------------- | ------------------------------------------------------------------------ |
| `vars` | Map<String, String> | Optional. Secret-specific environment variables for upstream operations. |
| `pull` | Boolean             | Optional. Whether to allow pulling this secret from the upstream.        |
| `push` | Boolean             | Optional. Whether to allow pushing this secret to the upstream.          |
