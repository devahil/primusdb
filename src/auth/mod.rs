/*!
# PrimusDB Authentication & Authorization Module

This module provides comprehensive authentication and authorization for PrimusDB,
including user management, role-based access control, API tokens, and cluster node authentication.

## Architecture

```
Authentication & Authorization Layer
══════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┐
│                    Authentication Service                         │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  User Management                                          │  │
│  │  • User creation and deletion                            │  │
│  │  • Password hashing (Argon2)                             │  │
│  │  • Password policies and expiration                      │  │
│  │  • Multi-factor authentication support                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  API Token System                                         │  │
│  │  • Token generation (cryptographically secure)           │  │
│  │  • Token expiration and revocation                        │  │
│  │  • Scoped tokens (limited permissions)                   │  │
│  │  • Token usage tracking and analytics                     │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Authorization (RBAC)                                     │  │
│  │  • Role definitions and hierarchy                        │  │
│  │  • Privilege-based access control                        │  │
│  │  • Resource-level permissions                              │  │
│  │  • Row-level security (segmentation)                     │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    Secure Access Layer                           │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Authentication Middleware                                 │  │
│  │  • Token validation                                       │  │
│  │  • Request authentication                                 │  │
│  │  • Session management                                     │  │
│  │  • Rate limiting by user                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Authorization Middleware                                 │  │
│  │  • Permission checking                                    │  │
│  │  • Resource access validation                             │  │
│  │  • Audit logging                                          │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                 Cluster Node Authentication                     │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Genesis Key System (Hyperledger-style)                   │  │
│  │  • Genesis key generation                                  │  │
│  │  • Node identity certificates                              │  │
│  │  • Cross-node authentication                              │  │
│  │  • Trust chain validation                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```
*/

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce as AesNonce};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Duration, Utc};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod cluster_auth;

pub use cluster_auth::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub segment_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub mfa_enabled: bool,
    pub mfa_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub privileges: Vec<Privilege>,
    pub parent_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Privilege {
    pub resource: ResourceType,
    pub actions: Vec<Action>,
    pub segment_filter: Option<String>,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    Columnar,
    Vector,
    Document,
    Relational,
    Cluster,
    Admin,
    All,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Read,
    Write,
    Delete,
    Create,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub parent_segment: Option<String>,
    pub data_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub scopes: Vec<TokenScope>,
    pub rate_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenScope {
    pub resource: ResourceType,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub require_auth: bool,
    pub min_password_length: u32,
    pub password_expiry_days: u32,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u32,
    pub token_expiry_hours: u32,
    pub session_timeout_minutes: u32,
    pub mfa_required_for_roles: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            require_auth: true,
            min_password_length: 8,
            password_expiry_days: 90,
            max_login_attempts: 5,
            lockout_duration_minutes: 30,
            token_expiry_hours: 8760,
            session_timeout_minutes: 60,
            mfa_required_for_roles: vec!["admin".to_string()],
        }
    }
}

pub struct AuthManager {
    config: AuthConfig,
    users: HashMap<String, User>,
    roles: HashMap<String, Role>,
    segments: HashMap<String, Segment>,
    tokens: HashMap<String, ApiToken>,
    token_by_hash: HashMap<String, String>,
    crypto: Arc<CryptoManager>,
    random: SystemRandom,
    login_attempts: HashMap<String, (u32, Option<DateTime<Utc>>)>,
}

struct CryptoManager {
    master_key: Vec<u8>,
    random: SystemRandom,
}

impl CryptoManager {
    fn new() -> Self {
        let mut master_key = vec![0u8; 32];
        let random = SystemRandom::new();
        let _ = random.fill(&mut master_key);
        Self { master_key, random }
    }

    fn encrypt_token(&self, token: &str) -> crate::Result<String> {
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to create cipher: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        self.random.fill(&mut nonce_bytes).map_err(|_| {
            crate::Error::CryptoError("Failed to generate nonce".to_string())
        })?;
        let nonce = AesNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, token.as_bytes())
            .map_err(|e| crate::Error::CryptoError(format!("Encryption failed: {}", e)))?;

        let tag_end = ciphertext.len() - 16;
        let actual_ciphertext = &ciphertext[..tag_end];
        let tag = &ciphertext[tag_end..];

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(tag);
        result.extend_from_slice(actual_ciphertext);

        Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &result))
    }

    fn decrypt_token(&self, encrypted: &str) -> crate::Result<String> {
        let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted)
            .map_err(|e| crate::Error::CryptoError(format!("Base64 decode failed: {}", e)))?;

        if data.len() < 28 {
            return Err(crate::Error::CryptoError("Invalid encrypted data".to_string()));
        }

        let nonce = &data[..12];
        let tag = &data[12..28];
        let ciphertext = &data[28..];

        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to create cipher: {}", e)))?;

        let nonce = AesNonce::from_slice(nonce);
        let mut combined = ciphertext.to_vec();
        combined.extend_from_slice(tag);

        let plaintext = cipher
            .decrypt(nonce, combined.as_slice())
            .map_err(|e| crate::Error::CryptoError(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| crate::Error::CryptoError(format!("UTF-8 decode failed: {}", e)))
    }
}

impl AuthManager {
    pub fn new(config: AuthConfig) -> crate::Result<Self> {
        let mut manager = Self {
            config,
            users: HashMap::new(),
            roles: HashMap::new(),
            segments: HashMap::new(),
            tokens: HashMap::new(),
            token_by_hash: HashMap::new(),
            crypto: Arc::new(CryptoManager::new()),
            random: SystemRandom::new(),
            login_attempts: HashMap::new(),
        };

        manager.init_default_roles()?;
        manager.create_admin_user()?;

        Ok(manager)
    }

    fn init_default_roles(&mut self) -> crate::Result<()> {
        self.roles.insert(
            "admin".to_string(),
            Role {
                id: "admin".to_string(),
                name: "Administrator".to_string(),
                description: "Full system access".to_string(),
                privileges: vec![Privilege {
                    resource: ResourceType::All,
                    actions: vec![Action::Read, Action::Write, Action::Delete, Action::Create, Action::Admin],
                    segment_filter: None,
                }],
                parent_role: None,
            },
        );

        self.roles.insert(
            "developer".to_string(),
            Role {
                id: "developer".to_string(),
                name: "Developer".to_string(),
                description: "Full data access with no admin".to_string(),
                privileges: vec![Privilege {
                    resource: ResourceType::All,
                    actions: vec![Action::Read, Action::Write, Action::Delete, Action::Create],
                    segment_filter: None,
                }],
                parent_role: None,
            },
        );

        self.roles.insert(
            "analyst".to_string(),
            Role {
                id: "analyst".to_string(),
                name: "Data Analyst".to_string(),
                description: "Read-only access to data".to_string(),
                privileges: vec![
                    Privilege {
                        resource: ResourceType::Columnar,
                        actions: vec![Action::Read],
                        segment_filter: None,
                    },
                    Privilege {
                        resource: ResourceType::Vector,
                        actions: vec![Action::Read],
                        segment_filter: None,
                    },
                    Privilege {
                        resource: ResourceType::Document,
                        actions: vec![Action::Read],
                        segment_filter: None,
                    },
                    Privilege {
                        resource: ResourceType::Relational,
                        actions: vec![Action::Read],
                        segment_filter: None,
                    },
                ],
                parent_role: None,
            },
        );

        self.roles.insert(
            "readonly".to_string(),
            Role {
                id: "readonly".to_string(),
                name: "Read Only".to_string(),
                description: "Minimal read access".to_string(),
                privileges: vec![Privilege {
                    resource: ResourceType::All,
                    actions: vec![Action::Read],
                    segment_filter: None,
                }],
                parent_role: None,
            },
        );

        self.roles.insert(
            "cluster_node".to_string(),
            Role {
                id: "cluster_node".to_string(),
                name: "Cluster Node".to_string(),
                description: "Node-to-node authentication".to_string(),
                privileges: vec![Privilege {
                    resource: ResourceType::Cluster,
                    actions: vec![Action::Read, Action::Write, Action::Admin],
                    segment_filter: None,
                }],
                parent_role: None,
            },
        );

        Ok(())
    }

    fn create_admin_user(&mut self) -> crate::Result<()> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password("admin123".as_bytes(), &salt)
            .map_err(|e| crate::Error::CryptoError(format!("Password hashing failed: {}", e)))?
            .to_string();

        let admin_user = User {
            id: "admin".to_string(),
            username: "admin".to_string(),
            password_hash,
            email: Some("admin@primusdb.local".to_string()),
            roles: vec!["admin".to_string()],
            segment_id: None,
            created_at: Utc::now(),
            last_login: None,
            is_active: true,
            mfa_enabled: false,
            mfa_secret: None,
        };

        self.users.insert("admin".to_string(), admin_user);
        Ok(())
    }

    pub fn create_user(
        &mut self,
        username: String,
        password: String,
        email: Option<String>,
        roles: Vec<String>,
        segment_id: Option<String>,
    ) -> crate::Result<String> {
        if username.len() < self.config.min_password_length as usize {
            return Err(crate::Error::ValidationError(
                "Username too short".to_string(),
            ));
        }

        if password.len() < self.config.min_password_length as usize {
            return Err(crate::Error::ValidationError(
                "Password too short".to_string(),
            ));
        }

        if self.users.contains_key(&username) {
            return Err(crate::Error::ValidationError(
                "User already exists".to_string(),
            ));
        }

        for role in &roles {
            if !self.roles.contains_key(role) {
                return Err(crate::Error::ValidationError(format!(
                    "Role {} does not exist",
                    role
                )));
            }
        }

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| crate::Error::CryptoError(format!("Password hashing failed: {}", e)))?
            .to_string();

        let user_id = format!("user_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let user = User {
            id: user_id.clone(),
            username,
            password_hash,
            email,
            roles,
            segment_id,
            created_at: Utc::now(),
            last_login: None,
            is_active: true,
            mfa_enabled: false,
            mfa_secret: None,
        };

        self.users.insert(user_id.clone(), user);
        Ok(user_id)
    }

    pub fn authenticate(&mut self, username: &str, password: &str) -> crate::Result<AuthResult> {
        if let Some((attempts, lockout)) = self.login_attempts.get(username) {
            if *attempts >= self.config.max_login_attempts {
                if let Some(lockout_until) = lockout {
                    if *lockout_until > Utc::now() {
                        return Err(crate::Error::AuthenticationError(
                            "Account temporarily locked".to_string(),
                        ));
                    }
                }
            }
        }

        let user = self
            .users
            .get(username)
            .ok_or_else(|| crate::Error::AuthenticationError("Invalid credentials".to_string()))?;

        if !user.is_active {
            return Err(crate::Error::AuthenticationError("Account is disabled".to_string()));
        }

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| crate::Error::CryptoError(format!("Invalid password hash: {}", e)))?;

        let argon2 = Argon2::default();
        let password_valid = argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        if !password_valid {
            let attempts = self.login_attempts.entry(username.to_string()).or_insert((0, None));
            attempts.0 += 1;
            if attempts.0 >= self.config.max_login_attempts {
                attempts.1 = Some(Utc::now() + Duration::minutes(self.config.lockout_duration_minutes as i64));
            }
            return Err(crate::Error::AuthenticationError("Invalid credentials".to_string()));
        }

        self.login_attempts.remove(username);

        let mut user = user.clone();
        user.last_login = Some(Utc::now());
        self.users.insert(username.to_string(), user.clone());

        let privileges = self.get_user_privileges(&user)?;

        Ok(AuthResult {
            user_id: user.id,
            username: user.username,
            roles: user.roles,
            segment_id: user.segment_id,
            privileges,
        })
    }

    pub fn create_api_token(
        &mut self,
        user_id: &str,
        name: String,
        scopes: Vec<TokenScope>,
        expires_in_hours: Option<u32>,
    ) -> crate::Result<(String, ApiToken)> {
        let user = self
            .users
            .get(user_id)
            .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;

        let mut token_bytes = vec![0u8; 32];
        self.random.fill(&mut token_bytes).map_err(|e| {
            crate::Error::CryptoError(format!("Failed to generate token: {}", e))
        })?;
        
        let raw_token = hex::encode(&token_bytes);
        let token_hash = {
            let mut hasher = Sha256::new();
            hasher.update(raw_token.as_bytes());
            hex::encode(hasher.finalize())
        };

        let expires_at = expires_in_hours.map(|hours| Utc::now() + Duration::hours(hours as i64));

        let token_id = format!("token_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        let token = ApiToken {
            id: token_id.clone(),
            user_id: user_id.to_string(),
            name,
            token_hash: token_hash.clone(),
            created_at: Utc::now(),
            expires_at,
            last_used: None,
            is_active: true,
            scopes,
            rate_limit: 1000,
        };

        self.tokens.insert(token_id.clone(), token.clone());
        self.token_by_hash.insert(token_hash, token_id);

        Ok((raw_token, token))
    }

    pub fn validate_token(&mut self, raw_token: &str) -> crate::Result<TokenValidation> {
        let token_hash = {
            let mut hasher = Sha256::new();
            hasher.update(raw_token.as_bytes());
            hex::encode(hasher.finalize())
        };

        let token_id = self
            .token_by_hash
            .get(&token_hash)
            .ok_or_else(|| crate::Error::AuthenticationError("Invalid token".to_string()))?;

        let token = self
            .tokens
            .get(token_id)
            .ok_or_else(|| crate::Error::AuthenticationError("Token not found".to_string()))?;

        if !token.is_active {
            return Err(crate::Error::AuthenticationError("Token is revoked".to_string()));
        }

        if let Some(expires_at) = token.expires_at {
            if expires_at < Utc::now() {
                return Err(crate::Error::AuthenticationError("Token expired".to_string()));
            }
        }

        let token_user_id = token.user_id.clone();
        let token_scopes = token.scopes.clone();
        
        let mut token = token.clone();
        token.last_used = Some(Utc::now());
        self.tokens.insert(token_id.clone(), token);

        let user = self
            .users
            .get(&token_user_id)
            .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;

        let privileges = self.get_user_privileges(user)?;

        Ok(TokenValidation {
            user_id: user.id.clone(),
            username: user.username.clone(),
            roles: user.roles.clone(),
            segment_id: user.segment_id.clone(),
            scopes: token_scopes,
            privileges,
        })
    }

    pub fn revoke_token(&mut self, token_id: &str) -> crate::Result<()> {
        let token = self
            .tokens
            .get_mut(token_id)
            .ok_or_else(|| crate::Error::ValidationError("Token not found".to_string()))?;

        if let Some(token_id_by_hash) = self.token_by_hash.remove(&token.token_hash) {
            let _ = token_id_by_hash;
        }

        token.is_active = false;
        Ok(())
    }

    pub fn list_user_tokens(&self, user_id: &str) -> Vec<ApiToken> {
        self.tokens
            .values()
            .filter(|t| t.user_id == user_id)
            .cloned()
            .collect()
    }

    fn get_user_privileges(&self, user: &User) -> crate::Result<Vec<Privilege>> {
        let mut privileges = Vec::new();

        for role_name in &user.roles {
            if let Some(role) = self.roles.get(role_name) {
                privileges.extend(role.privileges.clone());
            }
        }

        Ok(privileges)
    }

    pub fn check_permission(
        &self,
        validation: &TokenValidation,
        resource: ResourceType,
        action: Action,
    ) -> crate::Result<bool> {
        for scope in &validation.scopes {
            if scope.resource == ResourceType::All || scope.resource == resource {
                if scope.actions.contains(&action) || scope.actions.contains(&Action::Admin) {
                    return Ok(true);
                }
            }
        }

        for privilege in &validation.privileges {
            if privilege.resource == ResourceType::All || privilege.resource == resource {
                if privilege.actions.contains(&action) || privilege.actions.contains(&Action::Admin) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn create_segment(&mut self, name: String, description: String, parent_segment: Option<String>) -> crate::Result<String> {
        let segment_id = format!("seg_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        
        let segment = Segment {
            id: segment_id.clone(),
            name,
            description,
            parent_segment,
            data_retention_days: 90,
        };

        self.segments.insert(segment_id.clone(), segment);
        Ok(segment_id)
    }

    pub fn get_user(&self, user_id: &str) -> Option<User> {
        self.users.get(user_id).cloned()
    }

    pub fn list_users(&self) -> Vec<User> {
        self.users.values().cloned().collect()
    }

    pub fn list_roles(&self) -> Vec<Role> {
        self.roles.values().cloned().collect()
    }

    pub fn list_segments(&self) -> Vec<Segment> {
        self.segments.values().cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub segment_id: Option<String>,
    pub privileges: Vec<Privilege>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidation {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub segment_id: Option<String>,
    pub scopes: Vec<TokenScope>,
    pub privileges: Vec<Privilege>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scopes: Vec<TokenScope>,
    pub expires_in_hours: Option<u32>,
}

pub struct AuthService {
    auth_manager: Arc<RwLock<AuthManager>>,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> crate::Result<Self> {
        Ok(Self {
            auth_manager: Arc::new(RwLock::new(AuthManager::new(config)?)),
        })
    }

    pub async fn login(&self, request: LoginRequest) -> crate::Result<AuthResult> {
        let mut manager = self.auth_manager.write().await;
        manager.authenticate(&request.username, &request.password)
    }

    pub async fn create_token(&self, user_id: &str, request: CreateTokenRequest) -> crate::Result<(String, ApiToken)> {
        let mut manager = self.auth_manager.write().await;
        manager.create_api_token(user_id, request.name, request.scopes, request.expires_in_hours)
    }

    pub async fn validate_token(&self, token: &str) -> crate::Result<TokenValidation> {
        let mut manager = self.auth_manager.write().await;
        manager.validate_token(token)
    }

    pub async fn revoke_token(&self, token_id: &str) -> crate::Result<()> {
        let mut manager = self.auth_manager.write().await;
        manager.revoke_token(token_id)
    }

    pub async fn check_permission(&self, validation: &TokenValidation, resource: ResourceType, action: Action) -> crate::Result<bool> {
        let manager = self.auth_manager.read().await;
        manager.check_permission(validation, resource, action)
    }

    pub async fn create_user(&self, username: String, password: String, email: Option<String>, roles: Vec<String>, segment_id: Option<String>) -> crate::Result<String> {
        let mut manager = self.auth_manager.write().await;
        manager.create_user(username, password, email, roles, segment_id)
    }

    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        let manager = self.auth_manager.read().await;
        manager.get_user(user_id)
    }

    pub async fn list_users(&self) -> Vec<User> {
        let manager = self.auth_manager.read().await;
        manager.list_users()
    }

    pub async fn list_roles(&self) -> Vec<Role> {
        let manager = self.auth_manager.read().await;
        manager.list_roles()
    }

    pub async fn create_segment(&self, name: String, description: String, parent_segment: Option<String>) -> crate::Result<String> {
        let mut manager = self.auth_manager.write().await;
        manager.create_segment(name, description, parent_segment)
    }

    pub async fn list_user_tokens(&self, user_id: &str) -> Vec<ApiToken> {
        let manager = self.auth_manager.read().await;
        manager.list_user_tokens(user_id)
    }
}
