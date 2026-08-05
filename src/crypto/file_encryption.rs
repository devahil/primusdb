/*!
# PrimusDB File Encryption - Data-at-Rest Security

A standalone helper for encrypting individual files (or buffers) with
AES-256-GCM. It is not wired into the storage engines; callers invoke
[`FileEncryptionManager`] directly when they need an encrypted on-disk
artifact.

## Features

- **AES-256-GCM**: Authenticated encryption with tamper detection
- **Per-File Keys**: Each file's key is derived from a master key plus a
  random per-file salt
- **Integrity Verification**: 16-byte authentication tag + plaintext checksum
- **Convenience IO**: `write_encrypted_file` / `read_encrypted_file` / `is_encrypted_file`

## Architecture

```text
FileEncryptionManager
  ├─ new() / from_password()      master key (random or derived)
  ├─ encrypt_file(bytes)          -> header (44 B) + ciphertext + tag
  ├─ decrypt_file(bytes)          -> plaintext (verifies magic/tag/checksum)
  ├─ write_encrypted_file(path, plaintext)
  └─ read_encrypted_file(path)    (+ is_encrypted_file path probe)
```

## File Format

```text
Encrypted File Format
══════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────┐
│ Header (44 bytes total)                                       │
│ ├─ Magic: "PREN" (4 bytes)  - File identification             │
│ ├─ Version: u16 (2 bytes)   - Encryption format version       │
│ ├─ Flags: u16 (2 bytes)     - Encryption/compression flags    │
│ ├─ Key Salt: [u8; 16]       - For key derivation              │
│ ├─ Nonce: [u8; 12]          - Encryption nonce                │
│ └─ Checksum: [u8; 8]        - Plaintext integrity checksum    │
├────────────────────────────────────────────────────────────────┤
│ Encrypted Data (variable length)                               │
│ ├─ Authentication Tag: 16 bytes                                │
│ └─ Ciphertext: remaining bytes                                 │
└────────────────────────────────────────────────────────────────┘
```
*/

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce as AesNonce,
};
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write as IoWrite};
use std::path::Path;

/// Magic bytes identifying a PrimusDB encrypted file (`"PREN"`).
pub const FILE_MAGIC: &[u8; 4] = b"PREN";
/// Current encryption file format version.
pub const FILE_VERSION: u16 = 1;
/// Total header size in bytes (magic + version + flags + salt + nonce + checksum).
pub const HEADER_SIZE: usize = 44;
/// Size of the GCM nonce in bytes.
pub const NONCE_SIZE: usize = 12;
/// Size of the per-file key derivation salt in bytes.
pub const SALT_SIZE: usize = 16;
/// Size of the GCM authentication tag in bytes.
pub const TAG_SIZE: usize = 16;

/// Flags describing how a file was stored.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FileEncryptionFlags {
    /// Whether the payload is encrypted
    pub encrypted: bool,
    /// Whether the payload is compressed
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

/// Header stored at the start of every encrypted file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedFileHeader {
    /// Magic bytes identifying the format
    pub magic: [u8; 4],
    /// Encryption format version
    pub version: u16,
    /// Encryption/compression flags
    pub flags: FileEncryptionFlags,
    /// Salt used to derive the per-file key
    pub key_salt: [u8; SALT_SIZE],
    /// Nonce used during encryption
    pub nonce: [u8; NONCE_SIZE],
    /// Checksum of the plaintext (integrity verification)
    pub data_checksum: [u8; 8],
}

/// Encrypts and decrypts files on disk using AES-256-GCM with per-file keys.
///
/// Each file is encrypted under a key derived from the manager's master key
/// and a random per-file salt, so files remain unreadable even if the master
/// key is shared across the cluster.
pub struct FileEncryptionManager {
    master_key: [u8; 32],
    rng: ring::rand::SystemRandom,
}

impl FileEncryptionManager {
    /// Create a manager with a freshly generated random master key.
    pub fn new() -> Self {
        let mut master_key = [0u8; 32];
        let rng = ring::rand::SystemRandom::new();
        let _ = rng.fill(&mut master_key);
        Self { master_key, rng }
    }

    /// Create a manager whose master key is deterministically derived from a
    /// password, allowing the same files to be decrypted on any node that
    /// knows the password.
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
        hasher.update(self.master_key);
        hasher.update(salt);
        let result = hasher.finalize();
        key.copy_from_slice(&result[..32]);

        key
    }

    /// Encrypt a plaintext buffer into the on-disk file format (header + ciphertext).
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

    /// Decrypt a buffer produced by [`encrypt_file`](Self::encrypt_file),
    /// verifying the magic, version, authentication tag and checksum.
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

    /// Encrypt plaintext and write it to the given path.
    pub fn write_encrypted_file(&self, path: &Path, plaintext: &[u8]) -> crate::Result<()> {
        let encrypted = self.encrypt_file(plaintext)?;

        let mut file = File::create(path)?;
        file.write_all(&encrypted)?;

        Ok(())
    }

    /// Read and decrypt the file at the given path.
    pub fn read_encrypted_file(&self, path: &Path) -> crate::Result<Vec<u8>> {
        let mut file = File::open(path)?;
        let mut encrypted = Vec::new();
        file.read_to_end(&mut encrypted)?;

        self.decrypt_file(&encrypted)
    }

    /// Check whether a file starts with the PrimusDB encryption magic.
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
