use chrono::{TimeZone, Utc};
use cottage::*;
use std::path::Path;

#[test]
fn test_generate_preview_hcl() {
    let path = Path::new("test.hcl");
    let content = b"foo = \"bar\"\nservice \"web\" {\n  port = 8080\n}";
    let timestamp = Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let preview = generate_preview(path, content, None, None, &timestamp).unwrap();
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
    let preview = generate_preview(path, content, None, None, &timestamp).unwrap();
    assert_eq!(preview.format, PreviewFormat::Ini);
    assert!(preview.preview.contains("foo=2026-05-01T12:00:00+00:00"));
}

#[test]
fn test_generate_preview_dotenv() {
    let path = Path::new(".env");
    let content = b"FOO=BAR\nBAZ=QUX";
    let timestamp = Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap()
        .to_rfc3339();
    let preview = generate_preview(path, content, None, None, &timestamp).unwrap();
    assert_eq!(preview.format, PreviewFormat::Dotenv);
    assert!(preview.preview.contains("FOO=2026-05-01T12:00:00+00:00"));
}
