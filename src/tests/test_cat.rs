use crate::rotate::rotate;
use crate::{CatOptions, EncryptOptions, Project, cat, rotate_path, tee};
use age::secrecy::SecretSlice;
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
fn test_tee_encrypts_stdin_then_cat_decrypts() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (proj, opts) = setup(&proj_dir);
    let path = proj_dir.join("secret");

    let res = tee(
        proj.prefix(),
        &path,
        Some(SecretSlice::from(b"hello".to_vec())),
        &opts.encrypt_options,
    )
    .unwrap();
    assert!(res.is_some());
    assert!(proj.prefix().to_encrypted_path(&path).is_file());

    for p in [path.clone(), proj.prefix().to_encrypted_path(&path)] {
        let mut out = vec![];
        let res = cat(proj.prefix(), &p, &opts, &mut out).unwrap();
        assert!(res.is_none());
        assert_eq!(out, b"hello");
    }
}

#[test]
fn test_cat_decrypts() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (proj, opts) = setup(&proj_dir);
    let path = proj_dir.join("secret");

    tee(
        proj.prefix(),
        &path,
        Some(SecretSlice::from(b"data".to_vec())),
        &opts.encrypt_options,
    )
    .unwrap();

    let mut out = vec![];
    cat(proj.prefix(), &path, &opts, &mut out).unwrap();
    assert_eq!(out, b"data");
}

#[test]
fn test_cat_missing_secret_fails() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (proj, opts) = setup(&proj_dir);

    let err = cat(proj.prefix(), &proj_dir.join("nope"), &opts, vec![]).unwrap_err();
    assert!(err.to_string().contains("encrypted secret does not exist"));

    let missing_enc = proj.prefix().to_encrypted_path(&proj_dir.join("nope"));
    let err = cat(proj.prefix(), &missing_enc, &opts, vec![]).unwrap_err();
    assert!(err.to_string().contains("path does not exist"));
}

#[test]
fn test_tee_requires_input() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (proj, opts) = setup(&proj_dir);
    let path = proj_dir.join("secret");

    assert!(tee(proj.prefix(), &path, None, &opts.encrypt_options).is_err());
    assert!(
        tee(
            proj.prefix(),
            &path,
            Some(SecretSlice::from(vec![])),
            &opts.encrypt_options
        )
        .is_err()
    );
}

#[test]
fn test_rotate_reencrypts_in_memory() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (proj, opts) = setup(&proj_dir);
    let path = proj_dir.join("secret");

    tee(
        proj.prefix(),
        &path,
        Some(SecretSlice::from(b"hello".to_vec())),
        &opts.encrypt_options,
    )
    .unwrap();
    let enc = proj.prefix().to_encrypted_path(&path);
    let before = std::fs::read(&enc).unwrap();

    let res = rotate(proj.prefix(), &enc, &opts.encrypt_options)
        .unwrap()
        .unwrap();
    assert_eq!(res.input, enc);
    assert_ne!(std::fs::read(&enc).unwrap(), before);
    assert!(!path.exists());

    let mut out = vec![];
    cat(proj.prefix(), &path, &opts, &mut out).unwrap();
    assert_eq!(out, b"hello");
}

#[test]
fn test_rotate_path_dir() {
    let temp = assert_fs::TempDir::new().unwrap();
    let proj_dir = temp.child("proj");
    let (proj, opts) = setup(&proj_dir);

    for name in ["a", "b"] {
        tee(
            proj.prefix(),
            &proj_dir.join(name),
            Some(SecretSlice::from(name.as_bytes().to_vec())),
            &opts.encrypt_options,
        )
        .unwrap();
    }

    let results: Vec<_> = rotate_path(proj.prefix(), &proj_dir, &opts.encrypt_options)
        .collect::<anyhow::Result<_>>()
        .unwrap();
    assert_eq!(results.len(), 2);
}
