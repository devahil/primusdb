/*!
# PrimusDB File Encryption - Data-at-Rest Security

This module provides transparent file-level encryption for all storage engines,
ensuring that data files cannot be read or modified with hexadecimal editors.

## Features

- **Transparent Encryption**: Files are encrypted/decrypted automatically on read/write
- **AES-256-GCM**: Military-grade authenticated encryption
- **Per-File Keys**: Each file can have its own derived encryption key
- **Integrity Verification**: Tamper detection with authentication tags
- **Optional for Documents**: JSON documents can be stored encrypted or plaintext

## Architecture

```
File Encryption Layer
══════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┐
│                    Storage Engine Layer                           │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│  │  Columnar   │ │   Vector    │ │  Relational │            │
│  └──────────────┘ └──────────────┘ └──────────────┘            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   File Encryption Layer                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  EncryptedFileManager                                     │   │
│  │  • Auto-encrypt on write                                 │   │
│  │  • Auto-decrypt on read                                 │   │
│  │  • Key derivation per file                              │   │
│  │  • Tamper detection                                     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    File System                                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ .encbin  │ │ .encbin  │ │ .encbin  │ │ .json    │          │
│  │ (col)    │ │ (vec)    │ │ (rel)    │ │ (doc opt)│          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

## File Format

```
Encrypted File Format
══════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────┐
│ Header (16 bytes)                                              │
│ ├─ Magic: "PREN" (4 bytes) - File identification             │
│ ├─ Version: u16 (2 bytes) - Encryption format version         │
│ ├─ Flags: u16 (2 bytes) - Encryption options                  │
│ ├─ Key Salt: [u8; 16] (16 bytes) - For key derivation        │
│ ├─ Nonce: [u8; 12] (12 bytes) - Encryption nonce             │
│ └─ Reserved: [u8; 8] (8 bytes) - Future use                   │
├────────────────────────────────────────────────────────────────┤
│ Encrypted Data (variable length)                               │
│ ├─ Authentication Tag: 16 bytes                                │
│ └─ Ciphertext: remaining bytes                                 │
└────────────────────────────────────────────────────────────────┘
```
*/

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce as AesNonce,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write as IoWrite};
use std::path::Path;

pub const FILE_MAGIC: &[u8; 4] = b"PREN";
pub const FILE_VERSION: u16 = 1;
pub const HEADER_SIZE: usize = 44;
pub const NONCE_SIZE: usize = 12;
pub const SALT_SIZE: usize = 16;
pub const TAG_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FileEncryptionFlags {
    pub encrypted: bool,
    pub compressed: bool,
}

impl Default for FileEncryptionFlags {
    fn default() -> Self {
        Self {
            encrypted: true,
            compressed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedFileHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: FileEncryptionFlags,
    pub key_salt: [u8; SALT_SIZE],
    pub nonce: [u8; NONCE_SIZE],
    pub data_checksum: [u8; 8],
}

pub struct FileEncryptionManager {
    master_key: [u8; 32],
    rng: ring::rand::SystemRandom,
}

impl FileEncryptionManager {
    pub fn new() -> Self {
        let mut master_key = [0u8; 32];
        let rng = ring::rand::SystemRandom::new();
        let _ = rng.fill(&mut master_key);
        Self { master_key, rng }
    }

    pub fn from_password(password: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(b"primusdb_file_key_v1");
        let result = hasher.finalize();

        let mut master_key = [0u8; 32];
        master_key.copy_from_slice(&result[..32]);

        Self {
            master_key,
            rng: ring::rand::SystemRandom::new(),
        }
    }

    fn derive_file_key(&self, salt: &[u8; SALT_SIZE]) -> [u8; 32] {
        let mut key = [0u8; 32];

        let mut hasher = Sha256::new();
        hasher.update(&self.master_key);
        hasher.update(salt);
        let result = hasher.finalize();
        key.copy_from_slice(&result[..32]);

        key
    }

    pub fn encrypt_file(&self, plaintext: &[u8]) -> crate::Result<Vec<u8>> {
        let mut salt = [0u8; SALT_SIZE];
        self.rng
            .fill(&mut salt)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to generate salt: {}", e)))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        self.rng
            .fill(&mut nonce_bytes)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to generate nonce: {}", e)))?;

        let file_key = self.derive_file_key(&salt);
        let cipher = Aes256Gcm::new_from_slice(&file_key)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to create cipher: {}", e)))?;

        let nonce = AesNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| crate::Error::CryptoError(format!("Encryption failed: {}", e)))?;

        let tag_end = ciphertext.len() - TAG_SIZE;
        let actual_ciphertext = &ciphertext[..tag_end];
        let tag = &ciphertext[tag_end..];

        let mut data_checksum = [0u8; 8];
        let mut hasher = Sha256::new();
        hasher.update(plaintext);
        let hash_result = hasher.finalize();
        data_checksum.copy_from_slice(&hash_result[..8]);

        let mut output = Vec::with_capacity(HEADER_SIZE + ciphertext.len());

        output.extend_from_slice(FILE_MAGIC);
        output.extend_from_slice(&FILE_VERSION.to_le_bytes());

        let flags = FileEncryptionFlags::default();
        let flags_val = ((flags.compressed as u16) << 1) | (flags.encrypted as u16);
        output.extend_from_slice(&flags_val.to_le_bytes());

        output.extend_from_slice(&salt);
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&data_checksum);

        output.extend_from_slice(actual_ciphertext);
        output.extend_from_slice(tag);

        Ok(output)
    }

    pub fn decrypt_file(&self, encrypted: &[u8]) -> crate::Result<Vec<u8>> {
        if encrypted.len() < HEADER_SIZE + TAG_SIZE {
            return Err(crate::Error::CryptoError(
                "Encrypted file too short".to_string(),
            ));
        }

        let magic: [u8; 4] = encrypted[0..4].try_into().unwrap();
        if magic != *FILE_MAGIC {
            return Err(crate::Error::CryptoError("Invalid file format".to_string()));
        }

        let version = u16::from_le_bytes([encrypted[4], encrypted[5]]);
        if version != FILE_VERSION {
            return Err(crate::Error::CryptoError(format!(
                "Unsupported encryption version: {}",
                version
            )));
        }

        let flags_val = u16::from_le_bytes([encrypted[6], encrypted[7]]);
        let encrypted_flag = (flags_val & 1) != 0;
        if !encrypted_flag {
            return Ok(encrypted.to_vec());
        }

        let salt: [u8; SALT_SIZE] = encrypted[8..24]
            .try_into()
            .map_err(|_| crate::Error::CryptoError("Invalid salt".to_string()))?;

        let nonce_bytes: [u8; NONCE_SIZE] = encrypted[24..36]
            .try_into()
            .map_err(|_| crate::Error::CryptoError("Invalid nonce".to_string()))?;

        let stored_checksum = &encrypted[36..44];

        let ciphertext_with_tag = &encrypted[HEADER_SIZE..];
        let tag_start = ciphertext_with_tag.len() - TAG_SIZE;
        let actual_ciphertext = &ciphertext_with_tag[..tag_start];
        let tag = &ciphertext_with_tag[tag_start..];

        let file_key = self.derive_file_key(&salt);
        let cipher = Aes256Gcm::new_from_slice(&file_key)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to create cipher: {}", e)))?;

        let nonce = AesNonce::from_slice(&nonce_bytes);

        let mut combined = actual_ciphertext.to_vec();
        combined.extend_from_slice(tag);

        let plaintext = cipher
            .decrypt(nonce, combined.as_slice())
            .map_err(|e| crate::Error::CryptoError(format!("Decryption failed: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(&plaintext);
        let hash_result = hasher.finalize();
        let computed_checksum = &hash_result[..8];

        if computed_checksum != stored_checksum {
            return Err(crate::Error::CryptoError(
                "Data integrity check failed - file may be tampered".to_string(),
            ));
        }

        Ok(plaintext)
    }

    pub fn write_encrypted_file(&self, path: &Path, plaintext: &[u8]) -> crate::Result<()> {
        let encrypted = self.encrypt_file(plaintext)?;

        let mut file = File::create(path)?;
        file.write_all(&encrypted)?;

        Ok(())
    }

    pub fn read_encrypted_file(&self, path: &Path) -> crate::Result<Vec<u8>> {
        let mut file = File::open(path)?;
        let mut encrypted = Vec::new();
        file.read_to_end(&mut encrypted)?;

        self.decrypt_file(&encrypted)
    }

    pub fn is_encrypted_file(path: &Path) -> bool {
        if let Ok(mut file) = File::open(path) {
            let mut magic = [0u8; 4];
            if file.read_exact(&mut magic).is_ok() {
                return magic == *FILE_MAGIC;
            }
        }
        false
    }
}

impl Default for FileEncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let manager = FileEncryptionManager::new();
        let plaintext = b"Hello, PrimusDB encrypted world!";

        let encrypted = manager.encrypt_file(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = manager.decrypt_file(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_file_roundtrip() {
        let manager = FileEncryptionManager::new();
        let test_data = b"Test data for file encryption";
        let test_path = "/tmp/primusdb_test.enc";

        manager
            .write_encrypted_file(Path::new(test_path), test_data)
            .unwrap();

        assert!(FileEncryptionManager::is_encrypted_file(Path::new(
            test_path
        )));

        let decrypted = manager.read_encrypted_file(Path::new(test_path)).unwrap();
        assert_eq!(decrypted, test_data);

        std::fs::remove_file(test_path).ok();
    }

    #[test]
    fn test_tamper_detection() {
        let manager = FileEncryptionManager::new();
        let plaintext = b"Important data";

        let encrypted = manager.encrypt_file(plaintext).unwrap();

        let mut tampered = encrypted.clone();
        tampered[HEADER_SIZE + 10] ^= 0xFF;

        let result = manager.decrypt_file(&tampered);
        assert!(result.is_err());
    }
}
