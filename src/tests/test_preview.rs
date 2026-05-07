use crate::*;
use age::secrecy::SecretBox;
use chrono::{TimeZone, Utc};
use std::path::Path;

#[test]
fn test_generate_preview_hcl() {
    let path = Path::new("test.hcl");
    let content = b"foo = \"bar\"\nservice \"web\" {\n  port = 8080\n}";
    let timestamp = Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let preview = generate_preview(
        path,
        &SecretBox::new(content.to_vec().into()),
        None,
        None,
        &timestamp,
    )
    .unwrap();
    assert_eq!(preview.format, PreviewFormat::Hcl);
    assert!(preview.preview.contains("2026-05-01T12:00:00+00:00"));
}

#[test]
fn test_generate_preview_ini() {
    let path = Path::new("test.ini");
    let content = b"[section]\nfoo=bar";
    let timestamp = Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let preview = generate_preview(
        path,
        &SecretBox::new(content.to_vec().into()),
        None,
        None,
        &timestamp,
    )
    .unwrap();
    assert_eq!(preview.format, PreviewFormat::Ini);
    assert!(preview.preview.contains("foo=2026-05-01T12:00:00+00:00"));
}

#[test]
fn test_generate_preview_yaml_multi_doc() {
    let path = Path::new("test.yaml");
    let content = b"a: b\n---\nc: d";
    let timestamp = Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let preview = generate_preview(
        path,
        &SecretBox::new(content.to_vec().into()),
        None,
        None,
        &timestamp,
    )
    .unwrap();
    assert_eq!(preview.format, PreviewFormat::Yaml);
    assert!(preview.preview.contains("a: 2026-05-01T12:00:00+00:00"));
    assert!(preview.preview.contains("c: 2026-05-01T12:00:00+00:00"));
}

#[test]
fn test_generate_preview_jsonl() {
    let path = Path::new("test.json");
    let content = b"{\"a\": \"b\"}\n{\"c\": \"d\"}";
    let timestamp = Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let preview = generate_preview(
        path,
        &SecretBox::new(content.to_vec().into()),
        None,
        None,
        &timestamp,
    )
    .unwrap();
    assert_eq!(preview.format, PreviewFormat::Json);
    assert!(
        preview
            .preview
            .contains("{\"a\":\"2026-05-01T12:00:00+00:00\"}")
    );
    assert!(
        preview
            .preview
            .contains("{\"c\":\"2026-05-01T12:00:00+00:00\"}")
    );
}

#[test]
fn test_generate_preview_yaml_complex_keys() {
    let path = Path::new("test.yaml");
    let content = b"[key1, key2]: value";
    let timestamp = Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let preview = generate_preview(
        path,
        &SecretBox::new(content.to_vec().into()),
        None,
        None,
        &timestamp,
    )
    .unwrap();
    assert_eq!(preview.format, PreviewFormat::Yaml);
    // Values should be redacted
    assert!(preview.preview.contains("2026-05-01T12:00:00+00:00"));
    // Keys should NOT be redacted currently
    assert!(preview.preview.contains("key1"));
    assert!(preview.preview.contains("key2"));
}
