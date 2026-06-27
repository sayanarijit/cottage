use crate::identity::{load_identities, parse_identities_dir, parse_identities_path, parse_identity_file, Identity};
use crate::Project;
use age::secrecy::ExposeSecret;
use assert_fs::prelude::*;

const TEST_SSH_KEY: &str = "\
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACDH4c8b/sHwtUshZ21j3Zg3F4U6pG6VlM3gN4m4u0b7tQAAAIj22S969tkv
egAAAAtzc2gtZWQyNTUxOQAAACDH4c8b/sHwtUshZ21j3Zg3F4U6pG6VlM3gN4m4u0b7tQ
AAAED30fN0Ff15k49cuhL6+6dO4U6pG6VlM3gN4m4u0b7tQcfhzxv+wfC1SyFnbWPdmDcX
hTqkbpcUzeA3ibi7Rvu1AAAAEHRlc3RAZXhhbXBsZS5jb20BAgME
-----END OPENSSH PRIVATE KEY-----";

#[test]
fn test_parse_identity_file_single_age() {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("identity");
    let sk = age::x25519::Identity::generate();
    file.write_str(sk.to_string().expose_secret()).unwrap();

    let mut identities = parse_identity_file(file.path()).unwrap();
    let first = identities.next().unwrap();
    assert!(matches!(first, Identity::X25519(_)));
    assert!(identities.next().is_none());
}

#[test]
fn test_parse_identity_file_multiple_age() {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("identity");
    let sk1 = age::x25519::Identity::generate();
    let sk2 = age::x25519::Identity::generate();
    let content = format!(
        "{}\n{}",
        sk1.to_string().expose_secret(),
        sk2.to_string().expose_secret()
    );
    file.write_str(&content).unwrap();

    let mut identities = parse_identity_file(file.path()).unwrap();
    let first = identities.next().unwrap();
    assert!(matches!(first, Identity::X25519(_)));
    let second = identities.next().unwrap();
    assert!(matches!(second, Identity::X25519(_)));
    assert!(identities.next().is_none());
}

#[test]
fn test_parse_identity_file_age_with_comments_and_whitespace() {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("identity");
    let sk1 = age::x25519::Identity::generate();
    let sk2 = age::x25519::Identity::generate();
    let content = format!(
        "\n  \n# Comment 1\n   {}   \n\n# Comment 2\n{}",
        sk1.to_string().expose_secret(),
        sk2.to_string().expose_secret()
    );
    file.write_str(&content).unwrap();

    let mut identities = parse_identity_file(file.path()).unwrap();
    let first = identities.next().unwrap();
    assert!(matches!(first, Identity::X25519(_)));
    let second = identities.next().unwrap();
    assert!(matches!(second, Identity::X25519(_)));
    assert!(identities.next().is_none());
}

#[test]
fn test_parse_identity_file_ssh() {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("identity");
    file.write_str(TEST_SSH_KEY).unwrap();

    let mut identities = parse_identity_file(file.path()).unwrap();
    let first = identities.next().unwrap();
    assert!(matches!(first, Identity::Ssh(_)));
    assert!(identities.next().is_none());
}

#[test]
fn test_parse_identities_dir() {
    let temp = assert_fs::TempDir::new().unwrap();
    let sk1 = age::x25519::Identity::generate();
    let sk2 = age::x25519::Identity::generate();
    let sk3 = age::x25519::Identity::generate();

    // File 1: one age key
    temp.child("key1").write_str(sk1.to_string().expose_secret()).unwrap();

    // File 2: two age keys
    let content_2 = format!(
        "{}\n{}",
        sk2.to_string().expose_secret(),
        sk3.to_string().expose_secret()
    );
    temp.child("key2").write_str(&content_2).unwrap();

    // File 3: pub file (should be ignored)
    temp.child("key1.pub").write_str("ssh-ed25519 AAAAC3Nza...").unwrap();

    // Nested directory with a key
    let sub = temp.child("subdir");
    std::fs::create_dir_all(sub.path()).unwrap();
    let sk4 = age::x25519::Identity::generate();
    sub.child("key3").write_str(sk4.to_string().expose_secret()).unwrap();

    let identities: Vec<Identity> = parse_identities_dir(temp.path()).collect();
    assert_eq!(identities.len(), 4);
    for id in identities {
        assert!(matches!(id, Identity::X25519(_)));
    }
}

#[test]
fn test_parse_identities_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let sk = age::x25519::Identity::generate();
    let file = temp.child("key");
    file.write_str(sk.to_string().expose_secret()).unwrap();

    // Test with existing file path
    let mut identities_file = parse_identities_path(file.path()).unwrap();
    assert!(matches!(identities_file.next().unwrap(), Identity::X25519(_)));
    assert!(identities_file.next().is_none());

    // Test with existing directory path
    let mut identities_dir = parse_identities_path(temp.path()).unwrap();
    assert!(matches!(identities_dir.next().unwrap(), Identity::X25519(_)));
    assert!(identities_dir.next().is_none());

    // Test with non-existent path
    let identities_none = parse_identities_path(&temp.path().join("non-existent"));
    assert!(identities_none.is_none());
}

#[test]
fn test_load_identities_explicit() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj = Project::generate_test_project(temp.path());

    let sk1 = age::x25519::Identity::generate();
    let sk2 = age::x25519::Identity::generate();
    let file1 = temp.child("key1");
    let file2 = temp.child("key2");
    file1.write_str(sk1.to_string().expose_secret()).unwrap();
    file2.write_str(sk2.to_string().expose_secret()).unwrap();

    let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];
    let identities: Vec<Identity> = load_identities(&proj, paths).collect();
    assert_eq!(identities.len(), 2);
}

#[test]
fn test_load_identities_default_local() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj = Project::generate_test_project(temp.path());

    // Write identity file at local_identity_path
    let local_id_path = proj.identity_path();
    std::fs::create_dir_all(local_id_path.parent().unwrap()).unwrap();
    let sk = age::x25519::Identity::generate();
    std::fs::write(local_id_path, sk.to_string().expose_secret()).unwrap();

    let identities: Vec<Identity> = load_identities(&proj, vec![]).collect();
    assert_eq!(identities.len(), 1);
    assert!(matches!(identities[0], Identity::X25519(_)));
}
