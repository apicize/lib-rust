use apicize_lib::{ApicizeError, decrypt, encrypt, parameters::ParameterEncryption};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

const NONCE_LEN: usize = 12;
const SALT_LEN: usize = 16;

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let plaintext = "Hello, world!";
    let password = "my-secret-password";

    let encrypted = encrypt(plaintext, password, ParameterEncryption::Aes256Gcm).unwrap();
    let decrypted = decrypt(&encrypted, password, ParameterEncryption::Aes256Gcm).unwrap();

    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_produces_base64() {
    let encrypted = encrypt("test", "password", ParameterEncryption::Aes256Gcm).unwrap();
    assert!(BASE64.decode(&encrypted).is_ok());
}

#[test]
fn test_encrypt_different_each_time() {
    let encrypted1 = encrypt("same text", "password", ParameterEncryption::Aes256Gcm).unwrap();
    let encrypted2 = encrypt("same text", "password", ParameterEncryption::Aes256Gcm).unwrap();
    assert_ne!(encrypted1, encrypted2);
}

#[test]
fn test_decrypt_wrong_password() {
    let encrypted = encrypt("secret data", "correct-password", ParameterEncryption::Aes256Gcm).unwrap();
    let result = decrypt(&encrypted, "wrong-password", ParameterEncryption::Aes256Gcm);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_invalid_base64() {
    let result = decrypt("not-valid-base64!!!", "password", ParameterEncryption::Aes256Gcm);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_truncated_data() {
    let result = decrypt(&BASE64.encode(b"tooshort"), "password", ParameterEncryption::Aes256Gcm);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ApicizeError::Encryption { description } if description == "Encrypted data is too short"
    ));
}

#[test]
fn test_decrypt_tampered_ciphertext() {
    let encrypted = encrypt("important data", "password", ParameterEncryption::Aes256Gcm).unwrap();
    let mut bytes = BASE64.decode(&encrypted).unwrap();

    // Tamper with the ciphertext portion
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;

    let tampered = BASE64.encode(&bytes);
    let result = decrypt(&tampered, "password", ParameterEncryption::Aes256Gcm);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_tampered_checksum() {
    let encrypted = encrypt("important data", "password", ParameterEncryption::Aes256Gcm).unwrap();
    let mut bytes = BASE64.decode(&encrypted).unwrap();

    // Tamper with the checksum portion (starts after nonce + salt)
    bytes[NONCE_LEN + SALT_LEN] ^= 0xFF;

    let tampered = BASE64.encode(&bytes);
    let result = decrypt(&tampered, "password", ParameterEncryption::Aes256Gcm);
    assert!(result.is_err());
}

#[test]
fn test_empty_plaintext() {
    let encrypted = encrypt("", "password", ParameterEncryption::Aes256Gcm).unwrap();
    let decrypted = decrypt(&encrypted, "password", ParameterEncryption::Aes256Gcm).unwrap();
    assert_eq!(decrypted, "");
}

#[test]
fn test_unicode_plaintext() {
    let plaintext = "Hello 🌍 Привет мир 你好世界";
    let password = "unicode-password-🔑";

    let encrypted = encrypt(plaintext, password, ParameterEncryption::Aes256Gcm).unwrap();
    let decrypted = decrypt(&encrypted, password, ParameterEncryption::Aes256Gcm).unwrap();

    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_large_plaintext() {
    let plaintext: String = "A".repeat(100_000);
    let password = "password";

    let encrypted = encrypt(&plaintext, password, ParameterEncryption::Aes256Gcm).unwrap();
    let decrypted = decrypt(&encrypted, password, ParameterEncryption::Aes256Gcm).unwrap();

    assert_eq!(decrypted, plaintext);
}
