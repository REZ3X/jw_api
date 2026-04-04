use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;

use crate::error::AppError;

#[derive(Clone)]
pub struct CryptoService {
    master_key: Vec<u8>,
}

impl CryptoService {
    pub fn new(master_key_hex: &str) -> Result<Self, AppError> {
        let master_key = hex::decode(master_key_hex)
            .map_err(|e| AppError::EncryptionError(format!("Invalid master key hex: {}", e)))?;
        if master_key.len() != 32 {
            return Err(AppError::EncryptionError(
                "Master key must be 32 bytes (64 hex chars)".into(),
            ));
        }
        Ok(Self { master_key })
    }

    fn derive_user_key(&self, user_salt: &str) -> Result<[u8; 32], AppError> {
        let hk = Hkdf::<Sha256>::new(Some(user_salt.as_bytes()), &self.master_key);
        let mut okm = [0u8; 32];
        hk.expand(b"jw-e2e-encryption", &mut okm)
            .map_err(|e| AppError::EncryptionError(format!("HKDF expand failed: {}", e)))?;
        Ok(okm)
    }

    pub fn encrypt(&self, plaintext: &str, user_salt: &str) -> Result<String, AppError> {
        let key = self.derive_user_key(user_salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| AppError::EncryptionError(format!("Cipher init failed: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AppError::EncryptionError(format!("Encryption failed: {}", e)))?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(B64.encode(combined))
    }

    pub fn decrypt(&self, encrypted_b64: &str, user_salt: &str) -> Result<String, AppError> {
        let key = self.derive_user_key(user_salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| AppError::EncryptionError(format!("Cipher init failed: {}", e)))?;

        let combined = B64
            .decode(encrypted_b64)
            .map_err(|e| AppError::EncryptionError(format!("Base64 decode failed: {}", e)))?;

        if combined.len() < 12 {
            return Err(AppError::EncryptionError("Ciphertext too short".into()));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::EncryptionError(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::EncryptionError(format!("UTF-8 decode failed: {}", e)))
    }

    pub fn generate_user_salt() -> String {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        hex::encode(salt)
    }
}
