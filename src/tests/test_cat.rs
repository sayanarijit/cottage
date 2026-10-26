use crate::{CatOptions, EncryptOptions, Project, cat, to_encrypted_path};
use assert_fs::prelude::*;

fn setup(proj_dir: &assert_fs::fixture::ChildPath) -> (Project, CatOptions) {
    proj_dir.create_dir_all().unwrap();
    let proj = Project::generate_test_project(proj_dir);
    proj.init_test_recipients();
    let opts = CatOptions {
        encrypt_options: EncryptOptions {
            recipients: proj.load_test_recipients().collect(),
            identities: proj.load_test_identities().collect(),
            armor: true,
            skip_gitignore: true,
            skip_timestamps: true,
            skip_preview: true,
            force: false,
            identity_path: proj.identity_path().to_path_buf(),
            dry_run: false,
        },
        skip_verify_encrypted: false,
        skip_verify_recipients: false,
    };
    (proj, opts)
}

#[test]
fn test_cat_encrypts_stdin_then_decrypts() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (_proj, opts) = setup(&proj_dir);
    let path = proj_dir.join("secret");

    let mut out = vec![];
    let res = cat(&path, Some(b"hello".to_vec()), &opts, &mut out).unwrap();
    assert!(res.is_some());
    assert!(out.is_empty());
    assert!(to_encrypted_path(&path).is_file());

    for p in [path.clone(), to_encrypted_path(&path)] {
        let mut out = vec![];
        let res = cat(&p, None, &opts, &mut out).unwrap();
        assert!(res.is_none());
        assert_eq!(out, b"hello");
    }
}

#[test]
fn test_cat_empty_input_decrypts() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (_proj, opts) = setup(&proj_dir);
    let path = proj_dir.join("secret");

    cat(&path, Some(b"data".to_vec()), &opts, vec![]).unwrap();

    let mut out = vec![];
    cat(&path, Some(vec![]), &opts, &mut out).unwrap();
    assert_eq!(out, b"data");
}

#[test]
fn test_cat_missing_secret_fails() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (_proj, opts) = setup(&proj_dir);

    let err = cat(&proj_dir.join("nope"), None, &opts, vec![]).unwrap_err();
    assert!(err.to_string().contains("encrypted secret does not exist"));

    let missing_enc = to_encrypted_path(&proj_dir.join("nope"));
    let err = cat(&missing_enc, None, &opts, vec![]).unwrap_err();
    assert!(err.to_string().contains("path does not exist"));
}
