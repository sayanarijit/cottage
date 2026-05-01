use crate::{PreviewFormat, PreviewMetadata};
use chrono::{DateTime, Utc};
use std::path::Path;

fn redact_json(
    value: &mut serde_json::Value,
    old_value: Option<&serde_json::Value>,
    old_preview: Option<&serde_json::Value>,
    now_ts: &str,
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
                    now_ts,
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
                    now_ts,
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
                    *value = serde_json::Value::String(now_ts.to_string());
                }
            } else {
                *value = serde_json::Value::String(now_ts.to_string());
            }
        }
        serde_json::Value::Null => {}
    }
}

fn redact_yaml(
    value: &mut serde_yaml::Value,
    old_value: Option<&serde_yaml::Value>,
    old_preview: Option<&serde_yaml::Value>,
    now_ts: &str,
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
                    now_ts,
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
                    now_ts,
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
                    *value = serde_yaml::Value::String(now_ts.to_string());
                }
            } else {
                *value = serde_yaml::Value::String(now_ts.to_string());
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
                now_ts,
            );
        }
    }
}

fn redact_toml(
    value: &mut toml::Value,
    old_value: Option<&toml::Value>,
    old_preview: Option<&toml::Value>,
    now_ts: &str,
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
                    now_ts,
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
                    now_ts,
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
                    *value = toml::Value::String(now_ts.to_string());
                }
            } else {
                *value = toml::Value::String(now_ts.to_string());
            }
        }
    }
}

pub fn generate_preview(
    path: &Path,
    content: &[u8],
    old_content: Option<&[u8]>,
    old_preview: Option<&str>,
    timestamp: DateTime<Utc>,
) -> Option<PreviewMetadata> {
    let extension = path.extension()?.to_str()?;
    let now_ts = timestamp.to_rfc3339();

    match extension {
        "json" => {
            let mut value: serde_json::Value = serde_json::from_slice(content).ok()?;
            let old_value: Option<serde_json::Value> =
                old_content.and_then(|c| serde_json::from_slice(c).ok());
            let old_preview_value: Option<serde_json::Value> =
                old_preview.and_then(|p| serde_json::from_str(p).ok());

            redact_json(
                &mut value,
                old_value.as_ref(),
                old_preview_value.as_ref(),
                &now_ts,
            );
            Some(PreviewMetadata {
                format: PreviewFormat::Json,
                preview: serde_json::to_string_pretty(&value).ok()?,
            })
        }
        "yaml" | "yml" => {
            let mut value: serde_yaml::Value = serde_yaml::from_slice(content).ok()?;
            let old_value: Option<serde_yaml::Value> =
                old_content.and_then(|c| serde_yaml::from_slice(c).ok());
            let old_preview_value: Option<serde_yaml::Value> =
                old_preview.and_then(|p| serde_yaml::from_str(p).ok());

            redact_yaml(
                &mut value,
                old_value.as_ref(),
                old_preview_value.as_ref(),
                &now_ts,
            );
            Some(PreviewMetadata {
                format: PreviewFormat::Yaml,
                preview: serde_yaml::to_string(&value).ok()?,
            })
        }
        "toml" => {
            let content_str = std::str::from_utf8(content).ok()?;
            let mut value: toml::Value = toml::from_str(content_str).ok()?;

            let old_value: Option<toml::Value> = old_content
                .and_then(|c| std::str::from_utf8(c).ok())
                .and_then(|s| toml::from_str(s).ok());
            let old_preview_value: Option<toml::Value> =
                old_preview.and_then(|p| toml::from_str(p).ok());

            redact_toml(
                &mut value,
                old_value.as_ref(),
                old_preview_value.as_ref(),
                &now_ts,
            );
            Some(PreviewMetadata {
                format: PreviewFormat::Toml,
                preview: toml::to_string_pretty(&value).ok()?,
            })
        }
        _ => None,
    }
}
