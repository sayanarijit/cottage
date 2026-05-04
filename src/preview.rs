use age::secrecy::{ExposeSecret, SecretSlice};

use crate::{PreviewFormat, PreviewMetadata};
use std::path::Path;

fn redact_json(
    value: &mut serde_json::Value,
    old_value: Option<&serde_json::Value>,
    old_preview: Option<&serde_json::Value>,
    timestamp: &str,
) {
    match value {
        serde_json::Value::Object(map) => {
            let old_map = old_value.and_then(|v| v.as_object());
            let prev_map = old_preview.and_then(|v| v.as_object());
            for (k, v) in map.iter_mut() {
                redact_json(
                    v,
                    old_map.and_then(|m| m.get(k)),
                    prev_map.and_then(|m| m.get(k)),
                    timestamp,
                );
            }
        }
        serde_json::Value::Array(arr) => {
            let old_arr = old_value.and_then(|v| v.as_array());
            let prev_arr = old_preview.and_then(|v| v.as_array());
            for (i, v) in arr.iter_mut().enumerate() {
                redact_json(
                    v,
                    old_arr.and_then(|a| a.get(i)),
                    prev_arr.and_then(|a| a.get(i)),
                    timestamp,
                );
            }
        }
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_) => {
            if old_value == Some(value) {
                if let Some(prev) = old_preview {
                    *value = prev.clone();
                } else {
                    *value = serde_json::Value::String(timestamp.to_string());
                }
            } else {
                *value = serde_json::Value::String(timestamp.to_string());
            }
        }
        serde_json::Value::Null => {}
    }
}

fn redact_yaml(
    value: &mut serde_yaml::Value,
    old_value: Option<&serde_yaml::Value>,
    old_preview: Option<&serde_yaml::Value>,
    timestamp: &str,
) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            let old_map = old_value.and_then(|v| v.as_mapping());
            let prev_map = old_preview.and_then(|v| v.as_mapping());
            for (k, v) in map.iter_mut() {
                redact_yaml(
                    v,
                    old_map.and_then(|m| m.get(k)),
                    prev_map.and_then(|m| m.get(k)),
                    timestamp,
                );
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            let old_seq = old_value.and_then(|v| v.as_sequence());
            let prev_seq = old_preview.and_then(|v| v.as_sequence());
            for (i, v) in seq.iter_mut().enumerate() {
                redact_yaml(
                    v,
                    old_seq.and_then(|s| s.get(i)),
                    prev_seq.and_then(|s| s.get(i)),
                    timestamp,
                );
            }
        }
        serde_yaml::Value::String(_)
        | serde_yaml::Value::Number(_)
        | serde_yaml::Value::Bool(_) => {
            if old_value == Some(value) {
                if let Some(prev) = old_preview {
                    *value = prev.clone();
                } else {
                    *value = serde_yaml::Value::String(timestamp.to_string());
                }
            } else {
                *value = serde_yaml::Value::String(timestamp.to_string());
            }
        }
        serde_yaml::Value::Null => {}
        serde_yaml::Value::Tagged(tagged) => {
            let old_tagged = old_value.and_then(|v| {
                if let serde_yaml::Value::Tagged(t) = v {
                    Some(t)
                } else {
                    None
                }
            });
            let prev_tagged = old_preview.and_then(|v| {
                if let serde_yaml::Value::Tagged(t) = v {
                    Some(t)
                } else {
                    None
                }
            });
            redact_yaml(
                &mut tagged.value,
                old_tagged.map(|t| &t.value),
                prev_tagged.map(|t| &t.value),
                timestamp,
            );
        }
    }
}

fn redact_toml(
    value: &mut toml::Value,
    old_value: Option<&toml::Value>,
    old_preview: Option<&toml::Value>,
    timestamp: &str,
) {
    match value {
        toml::Value::Table(table) => {
            let old_table = old_value.and_then(|v| v.as_table());
            let prev_table = old_preview.and_then(|v| v.as_table());
            for (k, v) in table.iter_mut() {
                redact_toml(
                    v,
                    old_table.and_then(|t| t.get(k)),
                    prev_table.and_then(|t| t.get(k)),
                    timestamp,
                );
            }
        }
        toml::Value::Array(arr) => {
            let old_arr = old_value.and_then(|v| v.as_array());
            let prev_arr = old_preview.and_then(|v| v.as_array());
            for (i, v) in arr.iter_mut().enumerate() {
                redact_toml(
                    v,
                    old_arr.and_then(|a| a.get(i)),
                    prev_arr.and_then(|a| a.get(i)),
                    timestamp,
                );
            }
        }
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => {
            if old_value == Some(value) {
                if let Some(prev) = old_preview {
                    *value = prev.clone();
                } else {
                    *value = toml::Value::String(timestamp.to_string());
                }
            } else {
                *value = toml::Value::String(timestamp.to_string());
            }
        }
    }
}

fn redact_hcl(
    value: &mut hcl::Value,
    old_value: Option<&hcl::Value>,
    old_preview: Option<&hcl::Value>,
    timestamp: &str,
) {
    match value {
        hcl::Value::Object(map) => {
            let old_map = old_value.and_then(|v| v.as_object());
            let prev_map = old_preview.and_then(|v| v.as_object());
            for (k, v) in map.iter_mut() {
                redact_hcl(
                    v,
                    old_map.and_then(|m| m.get(k)),
                    prev_map.and_then(|m| m.get(k)),
                    timestamp,
                );
            }
        }
        hcl::Value::Array(arr) => {
            let old_arr = old_value.and_then(|v| v.as_array());
            let prev_arr = old_preview.and_then(|v| v.as_array());
            for (i, v) in arr.iter_mut().enumerate() {
                redact_hcl(
                    v,
                    old_arr.and_then(|a| a.get(i)),
                    prev_arr.and_then(|a| a.get(i)),
                    timestamp,
                );
            }
        }
        hcl::Value::String(_) | hcl::Value::Number(_) | hcl::Value::Bool(_) => {
            if old_value == Some(value) {
                if let Some(prev) = old_preview {
                    *value = prev.clone();
                } else {
                    *value = hcl::Value::String(timestamp.to_string());
                }
            } else {
                *value = hcl::Value::String(timestamp.to_string());
            }
        }
        hcl::Value::Null => {}
    }
}

fn redact_ini(
    ini: &mut ini::Ini,
    old_ini: Option<&ini::Ini>,
    old_preview: Option<&ini::Ini>,
    timestamp: &str,
) {
    for (section, prop) in ini.iter_mut() {
        let section_name = section;
        let old_prop = old_ini.and_then(|oi| oi.section(section_name));
        let prev_prop = old_preview.and_then(|op| op.section(section_name));

        for (k, v) in prop.iter_mut() {
            if old_prop.and_then(|p| p.get(k)) == Some(v) {
                if let Some(prev) = prev_prop.and_then(|p| p.get(k)) {
                    *v = prev.to_string();
                } else {
                    *v = timestamp.to_string();
                }
            } else {
                *v = timestamp.to_string();
            }
        }
    }
}

fn redact_dotenv(
    map: &mut indexmap::IndexMap<String, String>,
    old_map: Option<&indexmap::IndexMap<String, String>>,
    old_preview: Option<&indexmap::IndexMap<String, String>>,
    timestamp: &str,
) {
    for (k, v) in map.iter_mut() {
        if old_map.and_then(|m| m.get(k)) == Some(v) {
            if let Some(prev) = old_preview.and_then(|m| m.get(k)) {
                *v = prev.clone();
            } else {
                *v = timestamp.to_string();
            }
        } else {
            *v = timestamp.to_string();
        }
    }
}

pub fn generate_preview(
    path: &Path,
    content: &SecretSlice<u8>,
    old_content: Option<&SecretSlice<u8>>,
    old_preview: Option<&str>,
    timestamp: &str,
) -> Option<PreviewMetadata> {
    // WARNING: This finction should never error out exposing secrets.

    let filename = path.file_name()?.to_str()?;
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension {
        "json" => {
            let values: Vec<serde_json::Value> =
                if let Ok(v) = serde_json::from_slice(content.expose_secret()) {
                    vec![v]
                } else {
                    let s = str::from_utf8(content.expose_secret()).ok()?;
                    s.lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(|l| serde_json::from_str(l).ok())
                        .collect::<Option<Vec<_>>>()?
                };

            let old_values: Vec<serde_json::Value> = old_content
                .and_then(|c| {
                    if let Ok(v) = serde_json::from_slice(c.expose_secret()) {
                        Some(vec![v])
                    } else {
                        let s = str::from_utf8(c.expose_secret()).ok()?;
                        s.lines()
                            .filter(|l| !l.trim().is_empty())
                            .map(|l| serde_json::from_str(l).ok())
                            .collect::<Option<Vec<_>>>()
                    }
                })
                .unwrap_or_default();

            let old_preview_values: Vec<serde_json::Value> = old_preview
                .and_then(|p| {
                    if let Ok(v) = serde_json::from_str(p) {
                        Some(vec![v])
                    } else {
                        p.lines()
                            .filter(|l| !l.trim().is_empty())
                            .map(|l| serde_json::from_str(l).ok())
                            .collect::<Option<Vec<_>>>()
                    }
                })
                .unwrap_or_default();

            let mut preview = String::new();
            let len = values.len();
            for (i, mut value) in values.into_iter().enumerate() {
                redact_json(
                    &mut value,
                    old_values.get(i),
                    old_preview_values.get(i),
                    timestamp,
                );
                if i > 0 {
                    preview.push('\n');
                }
                if len == 1 {
                    preview.push_str(&serde_json::to_string_pretty(&value).ok()?);
                } else {
                    preview.push_str(&serde_json::to_string(&value).ok()?);
                }
            }

            Some(PreviewMetadata {
                format: PreviewFormat::Json,
                preview,
            })
        }
        "yaml" | "yml" => {
            use serde::Deserialize;
            let values: Vec<serde_yaml::Value> =
                serde_yaml::Deserializer::from_slice(content.expose_secret())
                    .map(|d| serde_yaml::Value::deserialize(d).ok())
                    .collect::<Option<Vec<_>>>()?;

            let old_values: Vec<serde_yaml::Value> = old_content
                .and_then(|c| {
                    serde_yaml::Deserializer::from_slice(c.expose_secret())
                        .map(|d| serde_yaml::Value::deserialize(d).ok())
                        .collect::<Option<Vec<_>>>()
                })
                .unwrap_or_default();

            let old_preview_values: Vec<serde_yaml::Value> = old_preview
                .and_then(|p| {
                    serde_yaml::Deserializer::from_str(p)
                        .map(|d| serde_yaml::Value::deserialize(d).ok())
                        .collect::<Option<Vec<_>>>()
                })
                .unwrap_or_default();

            let mut preview = String::new();
            for (i, mut value) in values.into_iter().enumerate() {
                redact_yaml(
                    &mut value,
                    old_values.get(i),
                    old_preview_values.get(i),
                    timestamp,
                );
                let doc_str = serde_yaml::to_string(&value).ok()?;
                if i > 0 {
                    preview.push_str("---\n");
                }
                preview.push_str(&doc_str);
            }

            Some(PreviewMetadata {
                format: PreviewFormat::Yaml,
                preview,
            })
        }
        "toml" => {
            let content_str = str::from_utf8(content.expose_secret()).ok()?;
            let mut value: toml::Value = toml::from_str(content_str).ok()?;

            let old_value: Option<toml::Value> = old_content
                .and_then(|c| str::from_utf8(c.expose_secret()).ok())
                .and_then(|s| toml::from_str(s).ok());
            let old_preview_value: Option<toml::Value> =
                old_preview.and_then(|p| toml::from_str(p).ok());

            redact_toml(
                &mut value,
                old_value.as_ref(),
                old_preview_value.as_ref(),
                timestamp,
            );
            Some(PreviewMetadata {
                format: PreviewFormat::Toml,
                preview: toml::to_string_pretty(&value).ok()?,
            })
        }
        "hcl" | "tf" => {
            let mut value: hcl::Value = hcl::from_slice(content.expose_secret()).ok()?;
            let old_value: Option<hcl::Value> =
                old_content.and_then(|c| hcl::from_slice(c.expose_secret()).ok());
            let old_preview_value: Option<hcl::Value> =
                old_preview.and_then(|p| hcl::from_str(p).ok());

            redact_hcl(
                &mut value,
                old_value.as_ref(),
                old_preview_value.as_ref(),
                timestamp,
            );
            Some(PreviewMetadata {
                format: PreviewFormat::Hcl,
                preview: hcl::to_string(&value).ok()?,
            })
        }
        "ini" | "cfg" | "conf" => {
            let content_str = str::from_utf8(content.expose_secret()).ok()?;
            let mut value = ini::Ini::load_from_str(content_str).ok()?;

            let old_value = old_content
                .and_then(|c| str::from_utf8(c.expose_secret()).ok())
                .and_then(|s| ini::Ini::load_from_str(s).ok());
            let old_preview_value = old_preview.and_then(|p| ini::Ini::load_from_str(p).ok());

            redact_ini(
                &mut value,
                old_value.as_ref(),
                old_preview_value.as_ref(),
                timestamp,
            );

            let mut buf = Vec::new();
            value.write_to(&mut buf).ok()?;
            Some(PreviewMetadata {
                format: PreviewFormat::Ini,
                preview: String::from_utf8(buf).ok()?,
            })
        }
        ext if ext == "env" || filename == ".env" => {
            let content_str = str::from_utf8(content.expose_secret()).ok()?;
            let mut value = parse_dotenv(content_str);

            let old_value = old_content
                .and_then(|c| str::from_utf8(c.expose_secret()).ok())
                .map(parse_dotenv);
            let old_preview_value = old_preview.map(parse_dotenv);

            redact_dotenv(
                &mut value,
                old_value.as_ref(),
                old_preview_value.as_ref(),
                timestamp,
            );

            let mut preview = String::new();
            for (k, v) in value {
                preview.push_str(&format!("{}={}\n", k, v));
            }

            Some(PreviewMetadata {
                format: PreviewFormat::Dotenv,
                preview,
            })
        }
        _ => None,
    }
}

fn parse_dotenv(content: &str) -> indexmap::IndexMap<String, String> {
    let mut map = indexmap::IndexMap::new();
    for (k, v) in dotenvy::Iter::new(content.as_bytes()).flatten() {
        map.insert(k, v);
    }
    map
}
