//! Multi-factor authentication (MFA) for PrimusDB users using time-based
//! one-time passwords (TOTP, RFC 6238).
//!
//! Enrollment yields a base32 secret, an `otpauth://` provisioning URL for QR
//! code display, and one-time backup codes. Verification compares the
//! submitted code against the current time step with a ±1 step tolerance to
//! absorb clock drift:
//!
//! ```text
//! MfaManager
//! +------------------+
//! | generate_secret  |  -> base32 secret
//! | generate_setup   |  -> MfaSetup (QR URL + backup codes)
//! | verify_code      |  <- code, checked against steps [-1, 0, +1]
//! +------------------+
//! ```

use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

const BASE32_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Data produced during MFA enrollment for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSetup {
    /// Base32 TOTP shared secret
    pub secret: String,
    /// `otpauth://` provisioning URL for QR code display
    pub qr_code_url: String,
    /// One-time backup codes for account recovery
    pub backup_codes: Vec<String>,
}

/// TOTP configuration controlling token format and timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaConfig {
    /// Issuer name shown in authenticator apps
    pub issuer: String,
    /// Number of digits in each generated code
    pub digits: u8,
    /// TOTP time step in seconds
    pub period: u32,
}

impl Default for MfaConfig {
    fn default() -> Self {
        Self {
            issuer: "PrimusDB".to_string(),
            digits: 6,
            period: 30,
        }
    }
}

/// Generates and verifies time-based one-time passwords (TOTP, RFC 6238).
pub struct MfaManager {
    config: MfaConfig,
}

impl MfaManager {
    /// Create an MFA manager with the given configuration.
    pub fn new(config: MfaConfig) -> Self {
        Self { config }
    }

    /// Generate a fresh base32 TOTP shared secret (20 random bytes).
    pub fn generate_secret(&self) -> String {
        let mut rng = rand::rng();
        let secret: Vec<u8> = (0..20).map(|_| rng.random()).collect();
        base32_encode(&secret)
    }

    /// Build the enrollment payload (QR provisioning URL and backup codes) for a user.
    pub fn generate_setup(&self, username: &str, secret: &str) -> MfaSetup {
        let qr_url = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&digits={}&period={}",
            self.config.issuer,
            username,
            secret,
            self.config.issuer,
            self.config.digits,
            self.config.period
        );

        let backup_codes = self.generate_backup_codes();

        MfaSetup {
            secret: secret.to_string(),
            qr_code_url: qr_url,
            backup_codes,
        }
    }

    /// Verify a submitted code against the current time step with a ±1 step
    /// tolerance to absorb clock drift between client and server.
    pub fn verify_code(&self, secret: &str, code: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let period = self.config.period as u64;
        let current_period = now / period;

        for offset in [-1i64, 0, 1] {
            let check_period = (current_period as i64 + offset) as u64;
            if self.verify_code_for_period(secret, code, check_period) {
                return true;
            }
        }
        false
    }

    fn verify_code_for_period(&self, secret: &str, code: &str, period: u64) -> bool {
        let secret_bytes = match base32_decode(secret) {
            Some(b) => b,
            None => return false,
        };

        let data = period.to_be_bytes();
        let mut mac = match HmacSha1::new_from_slice(&secret_bytes) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(&data);
        let result = mac.finalize().into_bytes();

        let offset = (result[19] & 0xf) as usize;
        let hash = ((result[offset] as u32 & 0x7f) << 24)
            | ((result[offset + 1] as u32) << 16)
            | ((result[offset + 2] as u32) << 8)
            | (result[offset + 3] as u32);

        let otp = hash % 10u32.pow(self.config.digits as u32);
        format!("{:0width$}", otp, width = self.config.digits as usize) == code
    }

    fn generate_backup_codes(&self) -> Vec<String> {
        let mut rng = rand::rng();
        (0..10)
            .map(|_| format!("{:08}", rng.random::<u32>()))
            .collect()
    }
}

fn base32_encode(data: &[u8]) -> String {
    let mut result = String::new();
    let mut bits = 0u32;
    let mut value = 0u32;

    for &byte in data {
        value = (value << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            result.push(BASE32_CHARS[((value >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        result.push(BASE32_CHARS[((value << (5 - bits)) & 0x1f) as usize] as char);
    }
    result
}

fn base32_decode(encoded: &str) -> Option<Vec<u8>> {
    let mut result = Vec::new();
    let mut bits = 0u32;
    let mut value = 0u32;

    for &byte in encoded.to_uppercase().as_bytes() {
        let val = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'2'..=b'7' => byte - b'2' + 26,
            b'=' => continue,
            _ => return None,
        };
        value = (value << 5) | val as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            result.push((value >> bits) as u8);
        }
    }
    Some(result)
}
