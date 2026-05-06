use age::Encryptor;
#[test]
fn test_empty() {
    let _ = Encryptor::with_recipients(vec![]);
}
