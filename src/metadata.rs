use age::secrecy::{ExposeSecret, SecretSlice};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretMetadata {
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChecksumMetadata {
    pub encrypted: String,
    pub decrypted: String,
    pub recipients: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreviewFormat {
    #[serde(rename = "yaml")]
    Yaml,

    #[serde(rename = "json")]
    Json,

    #[serde(rename = "toml")]
    Toml,

    #[serde(rename = "dotenv")]
    Dotenv,

    #[serde(rename = "ini")]
    Ini,

    #[serde(rename = "hcl")]
    Hcl,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewMetadata {
    pub format: PreviewFormat,
    pub preview: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub checksum: ChecksumMetadata,
    pub preview: Option<PreviewMetadata>,
    pub secret: SecretMetadata,
}

impl Metadata {
    pub fn read_from_path(path: &Path) -> Result<Self> {
        log::debug!("{}: reading metadata", path.display());
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("{}: could not read metadata file", path.display()))?;
        toml::from_str::<Metadata>(&content)
            .with_context(|| format!("{}: could not parse metadata", path.display()))
    }
}

pub fn make_checksum(data: &SecretSlice<u8>) -> String {
    format!("blake3:{}", blake3::hash(data.expose_secret()).to_hex())
}

pub fn verify_checksum(data: &SecretSlice<u8>, checksum: &str, path: &Path) -> Result<()> {
    log::debug!("{}: verifying checksum", path.display());
    match checksum.split_once(":") {
        Some(("blake3", cs)) => {
            let enc_checksum = blake3::hash(data.expose_secret()).to_hex();
            if enc_checksum.as_str() == cs {
                Ok(())
            } else {
                Err(anyhow!(
                    "{}: checksum mismatch: expected {}, got {}",
                    path.display(),
                    cs,
                    enc_checksum.as_str()
                ))
            }
        }
        Some((algo, _)) => Err(anyhow!("{algo}: unsupported checksum format in metadata",)),
        None => Err(anyhow!(
            "{}: invalid checksum format in metadata",
            path.display(),
        )),
    }
}
