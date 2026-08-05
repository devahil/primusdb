//! Local signing identity and ED25519 signatures.
//!
//! The signing service owns the node's integrity key pair. The private key is
//! persisted as PKCS#8 in the integrity store directory (development-mode
//! storage); production deployments are expected to reference an external
//! signing authority or KMS and disable local key material. Signatures are
//! produced over the canonical JSON bytes of each record so verification is
//! deterministic across nodes.

use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};

use super::errors::{IntegrityError, IntegrityResult};

/// Hex-encoded public key (32 bytes) used for signature verification.
pub type PublicKeyHex = String;

/// A persisted signing identity. `None` for the private key means the identity
/// is verification-only (a remote/authority signing setup).
pub struct SigningService {
    signer_id: String,
    key_pair: Option<Ed25519KeyPair>,
    public_key: PublicKeyHex,
}

impl SigningService {
    /// Loads an existing key pair from `path` or generates a new one and
    /// persists it. Returns a verification-only service when `path` is `None`.
    pub fn load_or_create(
        signer_id: &str,
        path: Option<&std::path::Path>,
    ) -> IntegrityResult<Self> {
        let (key_pair, public_key) = match path {
            Some(p) => {
                let existing = std::fs::read(p).ok();
                match existing {
                    Some(pkcs8) => match Ed25519KeyPair::from_pkcs8(&pkcs8) {
                        Ok(kp) => {
                            let pk = hex::encode(kp.public_key().as_ref());
                            (Some(kp), pk)
                        }
                        Err(_) => {
                            return Err(IntegrityError::SigningIdentityUnavailable(format!(
                                "corrupt key material at {}",
                                p.display()
                            )))
                        }
                    },
                    None => {
                        if let Some(parent) = p.parent() {
                            std::fs::create_dir_all(parent).map_err(|e| {
                                IntegrityError::SigningIdentityUnavailable(format!(
                                    "cannot create key dir: {}",
                                    e
                                ))
                            })?;
                        }
                        let rng = SystemRandom::new();
                        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|e| {
                            IntegrityError::SigningIdentityUnavailable(format!(
                                "key generation failed: {}",
                                e
                            ))
                        })?;
                        let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|e| {
                            IntegrityError::SigningIdentityUnavailable(format!(
                                "key parse failed: {}",
                                e
                            ))
                        })?;
                        let pk = hex::encode(kp.public_key().as_ref());
                        std::fs::write(p, pkcs8.as_ref()).map_err(|e| {
                            IntegrityError::SigningIdentityUnavailable(format!(
                                "key persistence failed: {}",
                                e
                            ))
                        })?;
                        (Some(kp), pk)
                    }
                }
            }
            None => (None, String::new()),
        };
        Ok(SigningService {
            signer_id: signer_id.to_string(),
            key_pair,
            public_key,
        })
    }

    /// Human-readable signer identity (node id / configured identity).
    pub fn signer_id(&self) -> &str {
        &self.signer_id
    }

    /// Hex public key. Empty when verification-only.
    pub fn public_key_hex(&self) -> &str {
        &self.public_key
    }

    /// True when this service can produce signatures locally.
    pub fn can_sign(&self) -> bool {
        self.key_pair.is_some()
    }

    /// Signs the given bytes, returning a base64-encoded ED25519 signature.
    pub fn sign(&self, data: &[u8]) -> IntegrityResult<String> {
        let kp = self.key_pair.as_ref().ok_or_else(|| {
            IntegrityError::SigningIdentityUnavailable(
                "local signing disabled; external authority required".to_string(),
            )
        })?;
        let sig = kp.sign(data);
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(sig.as_ref()))
    }

    /// Verifies `signature` (base64) over `data` against `public_key_hex`.
    pub fn verify(public_key_hex: &str, data: &[u8], signature: &str) -> IntegrityResult<bool> {
        let pk = hex::decode(public_key_hex).map_err(|_| {
            IntegrityError::SigningIdentityUnavailable("invalid public key encoding".to_string())
        })?;
        use base64::Engine;
        let sig = base64::engine::general_purpose::STANDARD
            .decode(signature)
            .map_err(|_| IntegrityError::SignatureVerificationFailed)?;
        let key = UnparsedPublicKey::new(&ED25519, &pk);
        Ok(key.verify(data, &sig).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_sign_verify_roundtrip() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("node_key");
        let svc = SigningService::load_or_create("node-1", Some(&key_path)).unwrap();
        assert!(svc.can_sign());
        let data = b"genesis payload";
        let sig = svc.sign(data).unwrap();
        assert!(SigningService::verify(svc.public_key_hex(), data, &sig).unwrap());
        assert!(!SigningService::verify(svc.public_key_hex(), b"tampered", &sig).unwrap());
    }

    #[test]
    fn test_reload_persists_identity() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("node_key");
        let svc1 = SigningService::load_or_create("node-1", Some(&key_path)).unwrap();
        let pk1 = svc1.public_key_hex().to_string();
        let data = b"stable identity";
        let sig = svc1.sign(data).unwrap();

        // Re-open from the same path: identity must survive restart.
        let svc2 = SigningService::load_or_create("node-1", Some(&key_path)).unwrap();
        assert_eq!(svc2.public_key_hex(), pk1);
        assert!(SigningService::verify(svc2.public_key_hex(), data, &sig).unwrap());
    }

    #[test]
    fn test_corrupt_key_is_error() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("bad_key");
        std::fs::write(&key_path, b"not a pkcs8 key").unwrap();
        let res = SigningService::load_or_create("node-1", Some(&key_path));
        assert!(matches!(
            res,
            Err(IntegrityError::SigningIdentityUnavailable(_))
        ));
    }

    #[test]
    fn test_verification_only_service() {
        let svc = SigningService::load_or_create("node-1", None).unwrap();
        assert!(!svc.can_sign());
        assert!(svc.sign(b"x").is_err());
    }
}
