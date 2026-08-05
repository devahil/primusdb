//! # Certificates — X.509 Certificate Generation
//!
//! Generates and loads CA, server, client and self-signed certificates using
//! the `rcgen` crate, writing PEM files to a configured output directory.
//!
//! ```text
//! generate_ca(out_dir, cn, days)            -> ca.crt + ca.key
//! generate_signed_cert(ca, config, out_dir) -> cert.pem + key.pem
//! generate_self_signed(config, out_dir)     -> selfsigned.pem + selfsigned.key
//! ```
//!
//! [`CertsCommands`] binds these functions to CLI subcommands, dispatched by
//! [`handle_certs_command`].
//!
//! ## Workflow
//!
//! ```text
//!   ┌────────────┐    ┌───────────────┐    ┌─────────────────────┐
//!   │ CreateCa   │ -> │ ca.crt,       │    │ CreateCert          │
//!   │ (bootstrap)│    │ ca.key        │ -> │ (load CA + sign)    │
//!   └────────────┘    └───────────────┘    │ -> cert.pem, key.pem│
//!                                          └─────────────────────┘
//!
//!   ┌──────────────────┐
//!   │ CreateSelfSigned │ -> selfsigned.pem + selfsigned.key
//!   └──────────────────┘
//! ```
//!
//! The PEM bytes for the CA certificate and key can also be loaded directly
//! with [`load_ca_pem`] and [`load_key_pem`] for programmatic signing.

use std::fs;
use std::path::{Path, PathBuf};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use time::OffsetDateTime;

/// Configuration for certificate generation
pub struct CertConfig {
    /// Common name embedded in the certificate's subject.
    pub common_name: String,
    /// Organization name embedded in the certificate's subject.
    pub organization: String,
    /// Validity period of the certificate in days.
    pub validity_days: u32,
    /// Subject alternative names (hosts/IPs) the certificate is valid for.
    pub hosts: Vec<String>,
    /// Whether this certificate should be a CA certificate.
    pub is_ca: bool,
    /// Whether to set the `serverAuth` extended key usage.
    pub is_server: bool,
    /// Whether to set the `clientAuth` extended key usage.
    pub is_client: bool,
}

impl Default for CertConfig {
    fn default() -> Self {
        Self {
            common_name: "PrimusDB".to_string(),
            organization: "PrimusDB".to_string(),
            validity_days: 3650,
            hosts: vec!["localhost".to_string(), "127.0.0.1".to_string()],
            is_ca: false,
            is_server: true,
            is_client: false,
        }
    }
}

/// Generate a CA certificate and key, saving them to files
pub fn generate_ca(
    out_dir: &Path,
    common_name: &str,
    validity_days: u32,
) -> Result<(PathBuf, PathBuf), String> {
    let key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate CA key: {}", e))?;

    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, common_name);
    params
        .distinguished_name
        .push(DnType::OrganizationName, "PrimusDB CA");
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    let not_before = OffsetDateTime::now_utc();
    let not_after = not_before + time::Duration::days(validity_days as i64);
    params.not_before = not_before;
    params.not_after = not_after;

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to sign CA cert: {}", e))?;

    fs::create_dir_all(out_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;

    let cert_path = out_dir.join("ca.crt");
    let key_path = out_dir.join("ca.key");

    fs::write(&cert_path, cert.pem()).map_err(|e| format!("Failed to write CA cert: {}", e))?;
    fs::write(&key_path, key_pair.serialize_pem())
        .map_err(|e| format!("Failed to write CA key: {}", e))?;

    Ok((cert_path, key_path))
}

/// Generate a certificate signed by the given CA
pub fn generate_signed_cert(
    ca_cert_pem: &str,
    ca_key_pem: &str,
    config: &CertConfig,
    out_dir: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    let ca_key =
        KeyPair::from_pem(ca_key_pem).map_err(|e| format!("Failed to parse CA key: {}", e))?;
    let issuer = Issuer::from_ca_cert_pem(ca_cert_pem, &ca_key)
        .map_err(|e| format!("Failed to parse CA cert: {}", e))?;

    let key_pair =
        KeyPair::generate().map_err(|e| format!("Failed to generate cert key: {}", e))?;

    let mut params = CertificateParams::new(config.hosts.clone())
        .map_err(|e| format!("Failed to create cert params: {}", e))?;
    params
        .distinguished_name
        .push(DnType::CommonName, &config.common_name);
    params
        .distinguished_name
        .push(DnType::OrganizationName, &config.organization);

    if config.is_server {
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    }
    if config.is_client {
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ClientAuth);
    }
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];

    let not_before = OffsetDateTime::now_utc();
    let not_after = not_before + time::Duration::days(config.validity_days as i64);
    params.not_before = not_before;
    params.not_after = not_after;

    let cert = params
        .signed_by(&key_pair, &issuer)
        .map_err(|e| format!("Failed to sign cert: {}", e))?;

    fs::create_dir_all(out_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;

    let cert_path = out_dir.join("cert.pem");
    let key_path = out_dir.join("key.pem");

    fs::write(&cert_path, cert.pem()).map_err(|e| format!("Failed to write cert: {}", e))?;
    fs::write(&key_path, key_pair.serialize_pem())
        .map_err(|e| format!("Failed to write key: {}", e))?;

    Ok((cert_path, key_path))
}

/// Generate a self-signed certificate
pub fn generate_self_signed(
    config: &CertConfig,
    out_dir: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    let key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate key: {}", e))?;

    let mut params = CertificateParams::new(config.hosts.clone())
        .map_err(|e| format!("Failed to create cert params: {}", e))?;
    params
        .distinguished_name
        .push(DnType::CommonName, &config.common_name);
    params
        .distinguished_name
        .push(DnType::OrganizationName, &config.organization);

    if config.is_server {
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    }
    if config.is_client {
        params
            .extended_key_usages
            .push(ExtendedKeyUsagePurpose::ClientAuth);
    }
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];

    let not_before = OffsetDateTime::now_utc();
    let not_after = not_before + time::Duration::days(config.validity_days as i64);
    params.not_before = not_before;
    params.not_after = not_after;

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to self-sign cert: {}", e))?;

    fs::create_dir_all(out_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;

    let cert_path = out_dir.join("selfsigned.pem");
    let key_path = out_dir.join("selfsigned.key");

    fs::write(&cert_path, cert.pem()).map_err(|e| format!("Failed to write cert: {}", e))?;
    fs::write(&key_path, key_pair.serialize_pem())
        .map_err(|e| format!("Failed to write key: {}", e))?;

    Ok((cert_path, key_path))
}

/// Load a CA cert from file, returning the PEM bytes
pub fn load_ca_pem(ca_path: &Path) -> Result<String, String> {
    fs::read_to_string(ca_path)
        .map_err(|e| format!("Failed to read CA cert from {}: {}", ca_path.display(), e))
}

/// Load key from file, returning the PEM bytes
pub fn load_key_pem(key_path: &Path) -> Result<String, String> {
    fs::read_to_string(key_path)
        .map_err(|e| format!("Failed to read key from {}: {}", key_path.display(), e))
}

/// CLI subcommands for certificate management
#[derive(clap::Subcommand)]
pub enum CertsCommands {
    /// Create a new Certificate Authority
    CreateCa {
        #[arg(long, default_value = ".")]
        out_dir: PathBuf,
        #[arg(long, default_value = "PrimusDB CA")]
        name: String,
        #[arg(long, default_value = "3650")]
        validity_days: u32,
    },
    /// Create a certificate signed by a CA (for servers or clients)
    CreateCert {
        #[arg(long)]
        ca_dir: PathBuf,
        #[arg(long, default_value = ".")]
        out_dir: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        hosts: Vec<String>,
        #[arg(long, default_value = "365")]
        validity_days: u32,
        #[arg(long)]
        server: bool,
        #[arg(long)]
        client: bool,
    },
    /// Create a self-signed certificate
    CreateSelfSigned {
        #[arg(long, default_value = ".")]
        out_dir: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        hosts: Vec<String>,
        #[arg(long, default_value = "365")]
        validity_days: u32,
    },
}

/// Dispatches a [`CertsCommands`] subcommand to the corresponding certificate
/// generator, printing progress to stdout.
pub async fn handle_certs_command(cmd: CertsCommands) -> Result<(), String> {
    match cmd {
        CertsCommands::CreateCa {
            out_dir,
            name,
            validity_days,
        } => {
            println!("🔐 Generating CA: {}", name);
            let (cert_path, key_path) = generate_ca(&out_dir, &name, validity_days)?;
            println!("✅ CA certificate: {}", cert_path.display());
            println!("✅ CA private key: {}", key_path.display());
            Ok(())
        }
        CertsCommands::CreateCert {
            ca_dir,
            out_dir,
            name,
            hosts,
            validity_days,
            server,
            client,
        } => {
            let ca_cert = load_ca_pem(&ca_dir.join("ca.crt"))?;
            let ca_key = load_key_pem(&ca_dir.join("ca.key"))?;
            let common_name = name.unwrap_or_else(|| "PrimusDB Node".to_string());
            let cert_hosts = if hosts.is_empty() {
                vec!["localhost".to_string(), "127.0.0.1".to_string()]
            } else {
                hosts
            };

            println!("🔐 Generating certificate: {}", common_name);
            let config = CertConfig {
                common_name,
                organization: "PrimusDB".to_string(),
                validity_days,
                hosts: cert_hosts,
                is_ca: false,
                is_server: server || (!client),
                is_client: client,
            };
            let (cert_path, key_path) = generate_signed_cert(&ca_cert, &ca_key, &config, &out_dir)?;
            println!("✅ Certificate: {}", cert_path.display());
            println!("✅ Private key: {}", key_path.display());
            Ok(())
        }
        CertsCommands::CreateSelfSigned {
            out_dir,
            name,
            hosts,
            validity_days,
        } => {
            let common_name = name.unwrap_or_else(|| "PrimusDB Self-Signed".to_string());
            let cert_hosts = if hosts.is_empty() {
                vec!["localhost".to_string(), "127.0.0.1".to_string()]
            } else {
                hosts
            };

            println!("🔐 Generating self-signed certificate: {}", common_name);
            let config = CertConfig {
                common_name,
                organization: "PrimusDB".to_string(),
                validity_days,
                hosts: cert_hosts,
                is_ca: false,
                is_server: true,
                is_client: true,
            };
            let (cert_path, key_path) = generate_self_signed(&config, &out_dir)?;
            println!("✅ Certificate: {}", cert_path.display());
            println!("✅ Private key: {}", key_path.display());
            Ok(())
        }
    }
}
