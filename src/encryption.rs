use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use pbkdf2::pbkdf2_hmac;
use rand::RngExt;
use sha2::{Digest, Sha256};

use crate::{ApicizeError, parameters::ParameterEncryption};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const CHECKSUM_LEN: usize = 32;
const KEY_LEN: usize = 32;
const PBKDF2_ITERATIONS: u32 = 100_000;

/// Encrypt plaintext using a password.
///
/// Derives an AES-256-GCM key from the password via PBKDF2 with a random salt,
/// encrypts the text, and returns a Base64 string containing the nonce, salt,
/// SHA-256 checksum of the plaintext, and ciphertext.
pub fn encrypt(plaintext: &str, password: &str, method: ParameterEncryption) -> Result<String, ApicizeError> {
    if method != ParameterEncryption::Aes256Gcm {
        return Err(ApicizeError::Encryption { description: "Invalid supported encyrption method".to_string() });
    }
    
    let mut rng = rand::rng();

    let mut salt = [0u8; SALT_LEN];
    rng.fill(&mut salt);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes);

    let checksum = Sha256::digest(plaintext.as_bytes());

    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| ApicizeError::Encryption {
            description: format!("Encryption failed: {}", e),
        })?;

    // Layout: nonce || salt || checksum || ciphertext
    let mut combined = Vec::with_capacity(NONCE_LEN + SALT_LEN + CHECKSUM_LEN + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&checksum);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypt a Base64-encoded encrypted string using a password.
///
/// Extracts the nonce, salt, and checksum, derives the key via PBKDF2,
/// decrypts the ciphertext, and validates the result against the checksum.
pub fn decrypt(encrypted: &str, password: &str, method: ParameterEncryption) -> Result<String, ApicizeError> {
    if method != ParameterEncryption::Aes256Gcm {
        return Err(ApicizeError::Encryption { description: "Unsupported encyrption method".to_string() });
    }

    let combined = BASE64
        .decode(encrypted)
        .map_err(|e| ApicizeError::Encryption {
            description: format!("Invalid Base64 input: {}", e),
        })?;

    let min_len = NONCE_LEN + SALT_LEN + CHECKSUM_LEN + 1;
    if combined.len() < min_len {
        return Err(ApicizeError::Encryption {
            description: "Encrypted data is too short".to_string(),
        });
    }

    let nonce_bytes = &combined[..NONCE_LEN];
    let salt = &combined[NONCE_LEN..NONCE_LEN + SALT_LEN];
    let checksum = &combined[NONCE_LEN + SALT_LEN..NONCE_LEN + SALT_LEN + CHECKSUM_LEN];
    let ciphertext = &combined[NONCE_LEN + SALT_LEN + CHECKSUM_LEN..];

    let key = derive_key(password, salt);
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| ApicizeError::Encryption {
            description: "Decryption failed: invalid password or corrupted data".to_string(),
        })?;

    let actual_checksum = Sha256::digest(&plaintext_bytes);
    if actual_checksum.as_slice() != checksum {
        return Err(ApicizeError::Encryption {
            description: "Checksum verification failed".to_string(),
        });
    }

    String::from_utf8(plaintext_bytes).map_err(|e| ApicizeError::Encryption {
        description: format!("Decrypted data is not valid UTF-8: {}", e),
    })
}

fn derive_key(password: &str, salt: &[u8]) -> Key<Aes256Gcm> {
    let mut key_bytes = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key_bytes);
    Key::<Aes256Gcm>::from(key_bytes)
}
