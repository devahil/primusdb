/*!
# PrimusDB Cryptographic Operations

Symmetric encryption, hashing and password hashing used to protect data at
rest. All encryption is authenticated (AEAD) so ciphertext tampering is
detectable.

## Module layout

```text
CryptoManager
  ├─ generate_data_key   -> EncryptionKey (always AES-256-GCM)
  ├─ encrypt / decrypt   -> EncryptedData { key_id, nonce, ciphertext, tag }
  │                        (AES-256-GCM or ChaCha20-Poly1305 per key)
  ├─ hash_data           -> SHA-256 | SHA3-256 | Blake3
  ├─ create/verify data integrity checksums
  ├─ hash_password / verify_password  -> Argon2id
  └─ rotate_keys         -> bump rotation counter, refresh active key set
```
*/

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce as AesNonce};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Manages encryption keys and performs authenticated encryption/decryption.
pub struct CryptoManager {
    key_rotation_counter: u64,
    active_keys: HashMap<String, EncryptionKey>,
    random: SystemRandom,
}

/// An encryption key with an identifier, creation/expiry timestamps and an
/// associated algorithm.
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    /// Unique key identifier used to reference the key when decrypting
    pub id: String,
    /// Raw key material (32 bytes for AES-256)
    pub key: Vec<u8>,
    /// When the key was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the key expires and should no longer be used
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Symmetric algorithm the key is used with
    pub algorithm: EncryptionAlgorithm,
}

/// Supported symmetric encryption algorithms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256 in GCM mode (authenticated encryption)
    AES256GCM,
    /// ChaCha20-Poly1305 (authenticated encryption)
    ChaCha20Poly1305,
}

/// Authenticated ciphertext produced by [`CryptoManager`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Id of the key that produced this ciphertext
    pub key_id: String,
    /// Unique nonce used during encryption
    pub nonce: Vec<u8>,
    /// Encrypted payload (without the authentication tag)
    pub ciphertext: Vec<u8>,
    /// Authentication tag used to detect tampering
    pub tag: Vec<u8>,
    /// Algorithm used for encryption
    pub algorithm: EncryptionAlgorithm,
}

/// Integrity metadata (checksum and hash algorithm) attached to encrypted
/// data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataIntegrity {
    /// Hex-encoded checksum of the data
    pub checksum: String,
    /// Hash algorithm used to compute the checksum
    pub hash_algorithm: HashAlgorithm,
    /// When the checksum was computed
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Supported hash algorithms for integrity checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// SHA-256
    SHA256,
    /// SHA-3 with a 256-bit digest
    SHA3_256,
    /// Blake3 (high performance)
    Blake3,
}

impl CryptoManager {
    /// Create a [`CryptoManager`], honouring the `encryption_enabled` flag.
    ///
    /// When encryption is disabled a no-op manager with no active keys is
    /// returned so that callers can share a single code path.
    pub fn new(config: &crate::SecurityConfig) -> crate::Result<Self> {
        if !config.encryption_enabled {
            // Return a no-op crypto manager when encryption is disabled
            return Ok(Self {
                key_rotation_counter: 0,
                active_keys: HashMap::new(),
                random: SystemRandom::new(),
            });
        }

        Ok(CryptoManager {
            key_rotation_counter: 0,
            active_keys: HashMap::new(),
            random: SystemRandom::new(),
        })
    }

    /// Generate a fresh 32-byte AES-256-GCM data key and register it as active.
    pub fn generate_data_key(&mut self) -> crate::Result<EncryptionKey> {
        let mut key_bytes = vec![0u8; 32];
        self.random.fill(&mut key_bytes).map_err(|e| {
            crate::Error::CryptoError(format!("Failed to generate data key: {}", e))
        })?;

        let key_id = format!(
            "key_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        let encryption_key = EncryptionKey {
            id: key_id.clone(),
            key: key_bytes,
            created_at: chrono::Utc::now(),
            expires_at,
            algorithm: EncryptionAlgorithm::AES256GCM,
        };

        self.active_keys.insert(key_id, encryption_key.clone());
        Ok(encryption_key)
    }

    /// Encrypt plaintext with the given key, returning authenticated ciphertext.
    pub fn encrypt(&self, plaintext: &[u8], key: &EncryptionKey) -> crate::Result<EncryptedData> {
        match key.algorithm {
            EncryptionAlgorithm::AES256GCM => self.encrypt_aes256_gcm(plaintext, key),
            EncryptionAlgorithm::ChaCha20Poly1305 => self.encrypt_chacha20_poly1305(plaintext, key),
        }
    }

    /// Decrypt authenticated ciphertext using the key registered under its id.
    pub fn decrypt(&self, encrypted_data: &EncryptedData) -> crate::Result<Vec<u8>> {
        let key = self
            .active_keys
            .get(&encrypted_data.key_id)
            .ok_or_else(|| crate::Error::CryptoError("Encryption key not found".to_string()))?;

        match encrypted_data.algorithm {
            EncryptionAlgorithm::AES256GCM => self.decrypt_aes256_gcm(encrypted_data, key),
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.decrypt_chacha20_poly1305(encrypted_data, key)
            }
        }
    }

    fn encrypt_aes256_gcm(
        &self,
        plaintext: &[u8],
        key: &EncryptionKey,
    ) -> crate::Result<EncryptedData> {
        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to create cipher: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        let rng = SystemRandom::new();
        rng.fill(&mut nonce_bytes)
            .map_err(|_| crate::Error::CryptoError("Failed to generate nonce".to_string()))?;
        let nonce = AesNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| crate::Error::CryptoError(format!("Encryption failed: {}", e)))?;

        // Split ciphertext and tag (in AES-GCM, the tag is part of the ciphertext)
        let tag_end = ciphertext.len() - 16; // GCM tag is 16 bytes
        let actual_ciphertext = &ciphertext[..tag_end];
        let tag = &ciphertext[tag_end..];

        Ok(EncryptedData {
            key_id: key.id.clone(),
            nonce: nonce_bytes.to_vec(),
            ciphertext: actual_ciphertext.to_vec(),
            tag: tag.to_vec(),
            algorithm: EncryptionAlgorithm::AES256GCM,
        })
    }

    fn decrypt_aes256_gcm(
        &self,
        encrypted_data: &EncryptedData,
        key: &EncryptionKey,
    ) -> crate::Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to create cipher: {}", e)))?;

        let nonce = AesNonce::from_slice(&encrypted_data.nonce);
        let mut combined = encrypted_data.ciphertext.clone();
        combined.extend_from_slice(&encrypted_data.tag);

        let plaintext = cipher
            .decrypt(nonce, combined.as_slice())
            .map_err(|e| crate::Error::CryptoError(format!("Decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    fn encrypt_chacha20_poly1305(
        &self,
        plaintext: &[u8],
        key: &EncryptionKey,
    ) -> crate::Result<EncryptedData> {
        // Implementation for ChaCha20-Poly1305
        let unbound_key = UnboundKey::new(&AES_256_GCM, &key.key).map_err(|e| {
            crate::Error::CryptoError(format!("Failed to create unbound key: {}", e))
        })?;

        let less_safe_key = LessSafeKey::new(unbound_key);

        let mut nonce_bytes = [0u8; 12];
        self.random
            .fill(&mut nonce_bytes)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to generate nonce: {}", e)))?;

        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let aad = Aad::empty();

        let mut ciphertext = plaintext.to_vec();
        less_safe_key
            .seal_in_place_append_tag(nonce, aad, &mut ciphertext)
            .map_err(|e| crate::Error::CryptoError(format!("ChaCha20 encryption failed: {}", e)))?;

        let tag_end = ciphertext.len() - 16;
        let actual_ciphertext = &ciphertext[..tag_end];
        let tag = &ciphertext[tag_end..];

        Ok(EncryptedData {
            key_id: key.id.clone(),
            nonce: nonce_bytes.to_vec(),
            ciphertext: actual_ciphertext.to_vec(),
            tag: tag.to_vec(),
            algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
        })
    }

    fn decrypt_chacha20_poly1305(
        &self,
        encrypted_data: &EncryptedData,
        key: &EncryptionKey,
    ) -> crate::Result<Vec<u8>> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, &key.key).map_err(|e| {
            crate::Error::CryptoError(format!("Failed to create unbound key: {}", e))
        })?;

        let less_safe_key = LessSafeKey::new(unbound_key);
        let nonce = Nonce::assume_unique_for_key(
            encrypted_data
                .nonce
                .as_slice()
                .try_into()
                .map_err(|_| crate::Error::CryptoError("Invalid nonce length".to_string()))?,
        );
        let aad = Aad::empty();

        let mut combined = encrypted_data.ciphertext.clone();
        combined.extend_from_slice(&encrypted_data.tag);

        less_safe_key
            .open_in_place(nonce, aad, &mut combined)
            .map_err(|e| crate::Error::CryptoError(format!("ChaCha20 decryption failed: {}", e)))?;

        // Remove tag from plaintext
        combined.truncate(combined.len() - 16);
        Ok(combined)
    }

    /// Hash data using the requested hash algorithm.
    pub fn hash_data(&self, data: &[u8], algorithm: HashAlgorithm) -> crate::Result<String> {
        match algorithm {
            HashAlgorithm::SHA256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                let hash = hasher.finalize();
                Ok(hex::encode(hash))
            }
            HashAlgorithm::SHA3_256 => {
                use sha3::Sha3_256;
                let mut hasher = Sha3_256::new();
                hasher.update(data);
                let hash = hasher.finalize();
                Ok(hex::encode(hash))
            }
            HashAlgorithm::Blake3 => {
                let hash = blake3::hash(data);
                Ok(hex::encode(hash.as_bytes()))
            }
        }
    }

    /// Verify that data matches a previously created [`DataIntegrity`] record.
    pub fn verify_data_integrity(
        &self,
        data: &[u8],
        integrity: &DataIntegrity,
    ) -> crate::Result<bool> {
        let computed_checksum = self.hash_data(data, integrity.hash_algorithm.clone())?;
        Ok(computed_checksum == integrity.checksum)
    }

    /// Compute a [`DataIntegrity`] record for data (Blake3 checksum).
    pub fn create_data_integrity(&self, data: &[u8]) -> crate::Result<DataIntegrity> {
        let checksum = self.hash_data(data, HashAlgorithm::Blake3)?;
        Ok(DataIntegrity {
            checksum,
            hash_algorithm: HashAlgorithm::Blake3,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Hash a password with Argon2, returning the PHC string for storage.
    pub fn hash_password(&self, password: &str) -> crate::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| crate::Error::CryptoError(format!("Password hashing failed: {}", e)))?;

        Ok(password_hash.to_string())
    }

    /// Verify a password against a stored Argon2 PHC hash.
    pub fn verify_password(&self, password: &str, hash: &str) -> crate::Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| crate::Error::CryptoError(format!("Invalid password hash: {}", e)))?;

        let argon2 = Argon2::default();

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(crate::Error::CryptoError(format!(
                "Password verification failed: {}",
                e
            ))),
        }
    }

    /// Rotate all active keys, expiring the current set and generating a fresh key.
    pub fn rotate_keys(&mut self) -> crate::Result<()> {
        println!("Rotating encryption keys");

        // Mark old keys for expiration
        for key in self.active_keys.values_mut() {
            key.expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
        }

        // Generate new keys
        let new_key = self.generate_data_key()?;
        println!("Generated new encryption key: {}", new_key.id);

        self.key_rotation_counter += 1;
        Ok(())
    }

    /// Report the health status of every registered key.
    pub fn get_key_status(&self) -> HashMap<String, KeyStatus> {
        let mut status = HashMap::new();
        let now = chrono::Utc::now();

        for (key_id, key) in &self.active_keys {
            let key_status = if key.expires_at <= now {
                KeyStatus::Expired
            } else if (key.expires_at - now) < chrono::Duration::hours(2) {
                KeyStatus::ExpiringSoon
            } else {
                KeyStatus::Active
            };

            status.insert(key_id.clone(), key_status);
        }

        status
    }
}

/// Lifecycle health status of an encryption key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyStatus {
    /// Key is valid and in use
    Active,
    /// Key expires within the configured rotation window
    ExpiringSoon,
    /// Key has passed its expiry time
    Expired,
}

pub mod file_encryption;

pub use file_encryption::{
    EncryptedFileHeader, FileEncryptionFlags, FileEncryptionManager, FILE_MAGIC, FILE_VERSION,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> crate::SecurityConfig {
        crate::SecurityConfig {
            encryption_enabled: true,
            key_rotation_interval: 86400,
            auth_required: false,
            mfa_enabled: false,
        }
    }

    fn disabled_config() -> crate::SecurityConfig {
        crate::SecurityConfig {
            encryption_enabled: false,
            key_rotation_interval: 86400,
            auth_required: false,
            mfa_enabled: false,
        }
    }

    // ── Key generation tests ─────────────────────────────────────────

    #[test]
    fn test_crypto_manager_creation_enabled() {
        let _cm = CryptoManager::new(&test_config()).unwrap();
    }

    #[test]
    fn test_crypto_manager_creation_disabled() {
        let _cm = CryptoManager::new(&disabled_config()).unwrap();
    }

    #[test]
    fn test_generate_data_key() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        assert_eq!(key.key.len(), 32);
        assert!(!key.id.is_empty());
        assert_eq!(key.algorithm, EncryptionAlgorithm::AES256GCM);
        let plaintext = b"test";
        let encrypted = cm.encrypt(plaintext, &key).unwrap();
        assert_eq!(encrypted.key_id, key.id);
    }

    #[test]
    fn test_generate_multiple_keys() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let k1 = cm.generate_data_key().unwrap();
        let k2 = cm.generate_data_key().unwrap();
        assert_ne!(k1.id, k2.id);
        let e1 = cm.encrypt(b"data1", &k1).unwrap();
        let e2 = cm.encrypt(b"data2", &k2).unwrap();
        assert_eq!(cm.decrypt(&e1).unwrap(), b"data1");
        assert_eq!(cm.decrypt(&e2).unwrap(), b"data2");
    }

    // ── Encrypt/decrypt round-trip (AES-256-GCM) ─────────────────────

    #[test]
    fn test_encrypt_decrypt_aes256gcm_roundtrip() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = b"Hello, PrimusDB AES-256-GCM!";
        let encrypted = cm.encrypt(plaintext, &key).unwrap();
        assert_eq!(encrypted.algorithm, EncryptionAlgorithm::AES256GCM);
        assert!(!encrypted.ciphertext.is_empty());
        assert_eq!(encrypted.nonce.len(), 12);
        assert_eq!(encrypted.tag.len(), 16);
        assert_eq!(encrypted.key_id, key.id);
        assert_ne!(encrypted.ciphertext, plaintext);

        let decrypted = cm.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = b"";
        let encrypted = cm.encrypt(plaintext, &key).unwrap();
        let decrypted = cm.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = vec![0xAB; 100_000];
        let encrypted = cm.encrypt(&plaintext, &key).unwrap();
        let decrypted = cm.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_binary_data() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = vec![0x00, 0xFF, 0xAA, 0x55, 0x01, 0xFE];
        let encrypted = cm.encrypt(&plaintext, &key).unwrap();
        let decrypted = cm.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    // ── Invalid key tests ────────────────────────────────────────────

    #[test]
    fn test_decrypt_wrong_key() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key1 = cm.generate_data_key().unwrap();
        let plaintext = b"secret data";
        let encrypted = cm.encrypt(plaintext, &key1).unwrap();

        let key2 = cm.generate_data_key().unwrap();
        let mut tampered = encrypted.clone();
        tampered.key_id = key2.id.clone();
        let result = cm.decrypt(&tampered);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_with_no_key() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let encrypted = EncryptedData {
            key_id: "nonexistent-key".to_string(),
            nonce: vec![0u8; 12],
            ciphertext: vec![1, 2, 3],
            tag: vec![0u8; 16],
            algorithm: EncryptionAlgorithm::AES256GCM,
        };
        let result = cm.decrypt(&encrypted);
        assert!(result.is_err());
    }

    // ── Tamper detection tests ───────────────────────────────────────

    #[test]
    fn test_tampered_ciphertext() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = b"important data";
        let mut encrypted = cm.encrypt(plaintext, &key).unwrap();
        if !encrypted.ciphertext.is_empty() {
            encrypted.ciphertext[0] ^= 0xFF;
        }
        let result = cm.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_tag() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = b"data with tag";
        let mut encrypted = cm.encrypt(plaintext, &key).unwrap();
        if !encrypted.tag.is_empty() {
            encrypted.tag[0] ^= 0xFF;
        }
        let result = cm.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_nonce() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = b"data with nonce";
        let mut encrypted = cm.encrypt(plaintext, &key).unwrap();
        if !encrypted.nonce.is_empty() {
            encrypted.nonce[0] ^= 0x01;
        }
        let result = cm.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_truncated_ciphertext() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let plaintext = b"truncate me";
        let mut encrypted = cm.encrypt(plaintext, &key).unwrap();
        if !encrypted.ciphertext.is_empty() {
            encrypted
                .ciphertext
                .truncate(encrypted.ciphertext.len() / 2);
        }
        let result = cm.decrypt(&encrypted);
        assert!(result.is_err());
    }

    // ── Hash / Integrity tests ───────────────────────────────────────

    #[test]
    fn test_hash_data_sha256() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let hash = cm.hash_data(b"test data", HashAlgorithm::SHA256).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_deterministic() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let h1 = cm.hash_data(b"same data", HashAlgorithm::SHA256).unwrap();
        let h2 = cm.hash_data(b"same data", HashAlgorithm::SHA256).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_data() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let h1 = cm.hash_data(b"data a", HashAlgorithm::SHA256).unwrap();
        let h2 = cm.hash_data(b"data b", HashAlgorithm::SHA256).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_create_and_verify_integrity() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let data = b"data to check";
        let integrity = cm.create_data_integrity(data).unwrap();
        assert!(cm.verify_data_integrity(data, &integrity).unwrap());
    }

    #[test]
    fn test_verify_integrity_tampered_data() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let data = b"original data";
        let integrity = cm.create_data_integrity(data).unwrap();
        let tampered = b"tampered data";
        assert!(!cm.verify_data_integrity(tampered, &integrity).unwrap());
    }

    #[test]
    fn test_hash_sha3() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let hash = cm.hash_data(b"sha3 test", HashAlgorithm::SHA3_256).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_blake3() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let hash = cm.hash_data(b"blake3 test", HashAlgorithm::Blake3).unwrap();
        assert_eq!(hash.len(), 64);
    }

    // ── Password hashing tests ───────────────────────────────────────

    #[test]
    fn test_hash_and_verify_password() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let password = "my_secure_password!";
        let hash = cm.hash_password(password).unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(cm.verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_verify_wrong_password() {
        let cm = CryptoManager::new(&test_config()).unwrap();
        let hash = cm.hash_password("correct_password").unwrap();
        assert!(!cm.verify_password("wrong_password", &hash).unwrap());
    }

    // ── Key rotation tests ────────────────────────────────────────────

    #[test]
    fn test_key_rotation() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let k1 = cm.generate_data_key().unwrap();
        let k2 = cm.generate_data_key().unwrap();

        assert_eq!(cm.decrypt(&cm.encrypt(b"a", &k1).unwrap()).unwrap(), b"a");
        assert_eq!(cm.decrypt(&cm.encrypt(b"b", &k2).unwrap()).unwrap(), b"b");

        cm.rotate_keys().unwrap();

        assert_eq!(cm.decrypt(&cm.encrypt(b"a", &k1).unwrap()).unwrap(), b"a");
        assert_eq!(cm.decrypt(&cm.encrypt(b"b", &k2).unwrap()).unwrap(), b"b");
    }

    // ── Key status tests ─────────────────────────────────────────────

    #[test]
    fn test_key_status_active() {
        let mut cm = CryptoManager::new(&test_config()).unwrap();
        let key = cm.generate_data_key().unwrap();
        let status = cm.get_key_status();
        assert!(matches!(status.get(&key.id), Some(KeyStatus::Active)));
    }

    // ── EncryptionAlgorithm tests ────────────────────────────────────

    #[test]
    fn test_encryption_algorithm_serde() {
        let alg = EncryptionAlgorithm::AES256GCM;
        let json = serde_json::to_string(&alg).unwrap();
        let deserialized: EncryptionAlgorithm = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EncryptionAlgorithm::AES256GCM);
    }

    #[test]
    fn test_hash_algorithm_serde() {
        let alg = HashAlgorithm::SHA256;
        let json = serde_json::to_string(&alg).unwrap();
        let deserialized: HashAlgorithm = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, HashAlgorithm::SHA256));
    }
}
