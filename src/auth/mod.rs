/*!
# PrimusDB Authentication & Authorization Module

This module provides comprehensive authentication and authorization for PrimusDB,
including user management, role-based access control, API tokens, and cluster node authentication.

## Architecture

```text
AuthService
  ├─ User Management: create/delete users, Argon2 password hashing,
  │    password expiry policy, MFA (TOTP) enrollment & verification
  ├─ API Tokens: secure token generation, expiry, revocation, scoped
  │    permissions, last-used tracking
  ├─ RBAC: roles (with optional parent hierarchy), privileges,
  │    resource types & actions, row-level segmentation
  └─ Cluster Auth: genesis-key trust anchor, node identity
       certificates, challenge/response authentication (cluster_auth)
```

### Module layout

- `mod.rs` — core auth types and `AuthManager`/`AuthService` implementations
- `mfa.rs` — TOTP (RFC 6238) multi-factor authentication
- `cluster_auth.rs` — genesis-key based cluster node authentication
*/

use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Duration, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod cluster_auth;
pub mod mfa;

pub use cluster_auth::*;
pub use mfa::{MfaConfig, MfaManager, MfaSetup};

/// A database user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Stable identifier for the user
    pub id: String,
    /// Unique login name
    pub username: String,
    /// Argon2 password hash (PHC string), never the plaintext password
    pub password_hash: String,
    /// Optional email address
    pub email: Option<String>,
    /// Names of the roles assigned to this user
    pub roles: Vec<String>,
    /// Optional segment used for row-level access control
    pub segment_id: Option<String>,
    /// When the account was created
    pub created_at: DateTime<Utc>,
    /// When the user last authenticated successfully
    pub last_login: Option<DateTime<Utc>>,
    /// Whether the account is enabled
    pub is_active: bool,
    /// Whether multi-factor authentication is enabled
    pub mfa_enabled: bool,
    /// Base32 TOTP shared secret for MFA
    pub mfa_secret: Option<String>,
}

/// A named set of privileges that can be assigned to users.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Stable role identifier
    pub id: String,
    /// Human readable role name
    pub name: String,
    /// Description of what the role is for
    pub description: String,
    /// Privileges granted by this role
    pub privileges: Vec<Privilege>,
    /// Optional parent role this role inherits from
    pub parent_role: Option<String>,
}

/// A set of actions allowed on a resource type, optionally restricted to a segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Privilege {
    /// The resource type the privilege applies to
    pub resource: ResourceType,
    /// The actions allowed on the resource
    pub actions: Vec<Action>,
    /// Optional segment restriction (row-level security)
    pub segment_filter: Option<String>,
}

/// The resource kinds that privileges and token scopes can target.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    /// Columnar storage engine
    Columnar,
    /// Vector storage engine
    Vector,
    /// Document storage engine
    Document,
    /// Relational storage engine
    Relational,
    /// Namespace management
    Namespace,
    /// Cluster management
    Cluster,
    /// Administrative operations
    Admin,
    /// All resource types (wildcard)
    All,
}

/// Actions that can be performed on a resource.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Read data from the resource
    Read,
    /// Write data to the resource
    Write,
    /// Delete data from the resource
    Delete,
    /// Create new objects in the resource
    Create,
    /// Administrative operations on the resource
    Admin,
}

/// A data segment used for row-level access control and data retention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Stable segment identifier
    pub id: String,
    /// Human readable segment name
    pub name: String,
    /// Description of the segment
    pub description: String,
    /// Optional parent segment for hierarchies
    pub parent_segment: Option<String>,
    /// Default data retention period in days
    pub data_retention_days: u32,
}

/// An API token granting scoped access to a user's resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    /// Stable token identifier
    pub id: String,
    /// Owner user id
    pub user_id: String,
    /// Human readable token name
    pub name: String,
    /// SHA-256 hash of the raw token (the raw value is never stored)
    pub token_hash: String,
    /// When the token was created
    pub created_at: DateTime<Utc>,
    /// Optional expiry time
    pub expires_at: Option<DateTime<Utc>>,
    /// When the token was last used
    pub last_used: Option<DateTime<Utc>>,
    /// Whether the token is currently active
    pub is_active: bool,
    /// Permission scopes granted to the token
    pub scopes: Vec<TokenScope>,
    /// Maximum requests allowed per rate-limit window
    pub rate_limit: u32,
}

/// A permission scope limiting a token to specific actions on a resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenScope {
    /// The resource type the scope covers
    pub resource: ResourceType,
    /// The actions allowed within the scope
    pub actions: Vec<Action>,
}

/// Security policy configuration for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Whether authentication is required at all
    pub require_auth: bool,
    /// Minimum length for passwords
    pub min_password_length: u32,
    /// Days before passwords must be rotated
    pub password_expiry_days: u32,
    /// Maximum failed login attempts before lockout
    pub max_login_attempts: u32,
    /// Lockout duration in minutes after exceeding the attempt limit
    pub lockout_duration_minutes: u32,
    /// Default API token expiry in hours
    pub token_expiry_hours: u32,
    /// Session timeout in minutes
    pub session_timeout_minutes: u32,
    /// Roles that require MFA to log in
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

/// Core authentication and authorization state.
///
/// Holds users, roles, segments and API tokens in memory and performs
/// password verification, token validation, permission checks and audit
/// logging.
pub struct AuthManager {
    config: AuthConfig,
    users: HashMap<String, User>,
    roles: HashMap<String, Role>,
    segments: HashMap<String, Segment>,
    tokens: HashMap<String, ApiToken>,
    token_by_hash: HashMap<String, String>,
    random: SystemRandom,
    login_attempts: HashMap<String, (u32, Option<DateTime<Utc>>)>,
    audit: Option<Arc<crate::system::audit::AuditLogger>>,
}

impl AuthManager {
    /// Create an [`AuthManager`], seeding the default roles and the admin user.
    pub fn new(config: AuthConfig) -> crate::Result<Self> {
        let mut manager = Self {
            config,
            users: HashMap::new(),
            roles: HashMap::new(),
            segments: HashMap::new(),
            tokens: HashMap::new(),
            token_by_hash: HashMap::new(),
            random: SystemRandom::new(),
            login_attempts: HashMap::new(),
            audit: None,
        };

        manager.init_default_roles()?;
        manager.create_admin_user()?;

        Ok(manager)
    }

    /// Set the audit logger for recording auth events.
    pub fn set_audit_logger(&mut self, logger: Arc<crate::system::audit::AuditLogger>) {
        self.audit = Some(logger);
    }

    fn log_audit(
        &self,
        event_type: &str,
        actor: &str,
        resource: &str,
        action: &str,
        detail: serde_json::Value,
        success: bool,
    ) {
        if let Some(ref audit) = self.audit {
            if let Err(e) = audit.log(event_type, actor, resource, action, detail, success) {
                tracing::warn!("Audit log failed: {}", e);
            }
        }
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
                    actions: vec![
                        Action::Read,
                        Action::Write,
                        Action::Delete,
                        Action::Create,
                        Action::Admin,
                    ],
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

    /// Create a new user with an Argon2-hashed password.
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

        let user_id = format!(
            "user_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let roles_clone = roles.clone();
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
        self.log_audit(
            "user.create",
            "system",
            &format!("user:{}", user_id),
            "create",
            serde_json::json!({"user_id": &user_id, "roles": &roles_clone}),
            true,
        );
        Ok(user_id)
    }

    /// Authenticate a user by username and password, applying lockout policy.
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
            return Err(crate::Error::AuthenticationError(
                "Account is disabled".to_string(),
            ));
        }

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| crate::Error::CryptoError(format!("Invalid password hash: {}", e)))?;

        let argon2 = Argon2::default();
        let password_valid = argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        if !password_valid {
            let attempts_count = {
                let attempts = self
                    .login_attempts
                    .entry(username.to_string())
                    .or_insert((0, None));
                attempts.0 += 1;
                if attempts.0 >= self.config.max_login_attempts {
                    attempts.1 = Some(
                        Utc::now() + Duration::minutes(self.config.lockout_duration_minutes as i64),
                    );
                }
                attempts.0
            };
            self.log_audit(
                "user.login",
                username,
                "auth",
                "authenticate",
                serde_json::json!({"success": false, "attempts": attempts_count}),
                false,
            );
            return Err(crate::Error::AuthenticationError(
                "Invalid credentials".to_string(),
            ));
        }

        self.login_attempts.remove(username);

        let mut user = user.clone();
        user.last_login = Some(Utc::now());
        self.users.insert(username.to_string(), user.clone());

        self.log_audit(
            "user.login",
            username,
            "auth",
            "authenticate",
            serde_json::json!({"success": true, "user_id": &user.id}),
            true,
        );

        let privileges = self.get_user_privileges(&user)?;

        Ok(AuthResult {
            user_id: user.id,
            username: user.username,
            roles: user.roles,
            segment_id: user.segment_id,
            privileges,
        })
    }

    /// Create a new API token for a user and return the raw token plus its record.
    pub fn create_api_token(
        &mut self,
        user_id: &str,
        name: String,
        scopes: Vec<TokenScope>,
        expires_in_hours: Option<u32>,
    ) -> crate::Result<(String, ApiToken)> {
        let _user = self
            .users
            .get(user_id)
            .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;

        let mut token_bytes = vec![0u8; 32];
        self.random
            .fill(&mut token_bytes)
            .map_err(|e| crate::Error::CryptoError(format!("Failed to generate token: {}", e)))?;

        let raw_token = hex::encode(&token_bytes);
        let token_hash = {
            let mut hasher = Sha256::new();
            hasher.update(raw_token.as_bytes());
            hex::encode(hasher.finalize())
        };

        let expires_at = expires_in_hours.map(|hours| Utc::now() + Duration::hours(hours as i64));

        let token_id = format!(
            "token_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

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

    /// Validate a raw API token and return the resolved identity and privileges.
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
            return Err(crate::Error::AuthenticationError(
                "Token is revoked".to_string(),
            ));
        }

        if let Some(expires_at) = token.expires_at {
            if expires_at < Utc::now() {
                return Err(crate::Error::AuthenticationError(
                    "Token expired".to_string(),
                ));
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

    /// Revoke an API token so it can no longer be used.
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

    /// List all API tokens belonging to a user.
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

    /// Check whether a validated identity has the requested action on a resource.
    pub fn check_permission(
        &self,
        validation: &TokenValidation,
        resource: ResourceType,
        action: Action,
    ) -> crate::Result<bool> {
        for scope in &validation.scopes {
            if (scope.resource == ResourceType::All || scope.resource == resource)
                && (scope.actions.contains(&action) || scope.actions.contains(&Action::Admin))
            {
                return Ok(true);
            }
        }

        for privilege in &validation.privileges {
            if (privilege.resource == ResourceType::All || privilege.resource == resource)
                && (privilege.actions.contains(&action)
                    || privilege.actions.contains(&Action::Admin))
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Create a new data segment for row-level access control.
    pub fn create_segment(
        &mut self,
        name: String,
        description: String,
        parent_segment: Option<String>,
    ) -> crate::Result<String> {
        let segment_id = format!(
            "seg_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

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

    /// Fetch a user by id.
    pub fn get_user(&self, user_id: &str) -> Option<User> {
        self.users.get(user_id).cloned()
    }

    /// List all users.
    pub fn list_users(&self) -> Vec<User> {
        self.users.values().cloned().collect()
    }

    /// List all roles.
    pub fn list_roles(&self) -> Vec<Role> {
        self.roles.values().cloned().collect()
    }

    /// List all segments.
    pub fn list_segments(&self) -> Vec<Segment> {
        self.segments.values().cloned().collect()
    }
}

/// Result of a successful authentication, containing the resolved identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// Authenticated user id
    pub user_id: String,
    /// Authenticated username
    pub username: String,
    /// Roles resolved for the user
    pub roles: Vec<String>,
    /// Optional segment restriction for the user
    pub segment_id: Option<String>,
    /// Privileges aggregated from the user's roles
    pub privileges: Vec<Privilege>,
}

/// Result of validating an API token, containing the resolved identity and scopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidation {
    /// Owning user id
    pub user_id: String,
    /// Owning username
    pub username: String,
    /// Roles resolved for the user
    pub roles: Vec<String>,
    /// Optional segment restriction for the user
    pub segment_id: Option<String>,
    /// Scopes granted by the token itself
    pub scopes: Vec<TokenScope>,
    /// Privileges aggregated from the user's roles
    pub privileges: Vec<Privilege>,
}

/// Credentials submitted to the login endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    /// Login name of the user
    pub username: String,
    /// Plaintext password to verify
    pub password: String,
}

/// Payload for creating a scoped API token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    /// Human readable name for the token
    pub name: String,
    /// Permission scopes granted to the token
    pub scopes: Vec<TokenScope>,
    /// Optional expiry in hours (defaults to the configured policy)
    pub expires_in_hours: Option<u32>,
}

/// Async wrapper around [`AuthManager`] and the MFA manager.
pub struct AuthService {
    auth_manager: Arc<RwLock<AuthManager>>,
    mfa_manager: Arc<MfaManager>,
}

impl AuthService {
    /// Create an [`AuthService`] with default MFA configuration.
    pub fn new(config: AuthConfig) -> crate::Result<Self> {
        Ok(Self {
            auth_manager: Arc::new(RwLock::new(AuthManager::new(config)?)),
            mfa_manager: Arc::new(MfaManager::new(MfaConfig::default())),
        })
    }

    /// Create an [`AuthService`] with a custom MFA configuration.
    pub fn new_with_mfa(config: AuthConfig, mfa_config: MfaConfig) -> crate::Result<Self> {
        Ok(Self {
            auth_manager: Arc::new(RwLock::new(AuthManager::new(config)?)),
            mfa_manager: Arc::new(MfaManager::new(mfa_config)),
        })
    }

    /// Authenticate a user with username and password.
    pub async fn login(&self, request: LoginRequest) -> crate::Result<AuthResult> {
        let mut manager = self.auth_manager.write().await;
        manager.authenticate(&request.username, &request.password)
    }

    /// Create a scoped API token for a user.
    pub async fn create_token(
        &self,
        user_id: &str,
        request: CreateTokenRequest,
    ) -> crate::Result<(String, ApiToken)> {
        let mut manager = self.auth_manager.write().await;
        manager.create_api_token(
            user_id,
            request.name,
            request.scopes,
            request.expires_in_hours,
        )
    }

    /// Validate a raw API token.
    pub async fn validate_token(&self, token: &str) -> crate::Result<TokenValidation> {
        let mut manager = self.auth_manager.write().await;
        manager.validate_token(token)
    }

    /// Revoke an API token.
    pub async fn revoke_token(&self, token_id: &str) -> crate::Result<()> {
        let mut manager = self.auth_manager.write().await;
        manager.revoke_token(token_id)
    }

    /// Check a permission for a validated identity.
    pub async fn check_permission(
        &self,
        validation: &TokenValidation,
        resource: ResourceType,
        action: Action,
    ) -> crate::Result<bool> {
        let manager = self.auth_manager.read().await;
        manager.check_permission(validation, resource, action)
    }

    /// Create a new user with the given roles and optional segment.
    pub async fn create_user(
        &self,
        username: String,
        password: String,
        email: Option<String>,
        roles: Vec<String>,
        segment_id: Option<String>,
    ) -> crate::Result<String> {
        let mut manager = self.auth_manager.write().await;
        manager.create_user(username, password, email, roles, segment_id)
    }

    /// Fetch a user by id.
    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        let manager = self.auth_manager.read().await;
        manager.get_user(user_id)
    }

    /// List all users.
    pub async fn list_users(&self) -> Vec<User> {
        let manager = self.auth_manager.read().await;
        manager.list_users()
    }

    /// List all roles.
    pub async fn list_roles(&self) -> Vec<Role> {
        let manager = self.auth_manager.read().await;
        manager.list_roles()
    }

    /// Create a new data segment.
    pub async fn create_segment(
        &self,
        name: String,
        description: String,
        parent_segment: Option<String>,
    ) -> crate::Result<String> {
        let mut manager = self.auth_manager.write().await;
        manager.create_segment(name, description, parent_segment)
    }

    /// List all API tokens for a user.
    pub async fn list_user_tokens(&self, user_id: &str) -> Vec<ApiToken> {
        let manager = self.auth_manager.read().await;
        manager.list_user_tokens(user_id)
    }

    /// Begin MFA enrollment for a user, storing the shared secret.
    pub async fn mfa_setup(&self, username: &str) -> crate::Result<MfaSetup> {
        let secret = self.mfa_manager.generate_secret();
        let setup = self.mfa_manager.generate_setup(username, &secret);

        let mut manager = self.auth_manager.write().await;
        let mut user = manager
            .get_user(username)
            .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;
        user.mfa_secret = Some(secret);
        manager.users.insert(username.to_string(), user);

        Ok(setup)
    }

    /// Verify an MFA code for a user, enabling MFA on success.
    pub async fn mfa_verify(&self, username: &str, code: &str) -> crate::Result<bool> {
        let manager = self.auth_manager.read().await;
        let user = manager
            .get_user(username)
            .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;

        let secret = user
            .mfa_secret
            .ok_or_else(|| crate::Error::ValidationError("MFA not configured".to_string()))?;

        if self.mfa_manager.verify_code(&secret, code) {
            drop(manager);
            let mut manager = self.auth_manager.write().await;
            let mut user = manager
                .get_user(username)
                .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;
            user.mfa_enabled = true;
            manager.users.insert(username.to_string(), user);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Disable MFA for a user after verifying a valid code.
    pub async fn mfa_disable(&self, username: &str, code: &str) -> crate::Result<()> {
        let manager = self.auth_manager.read().await;
        let user = manager
            .get_user(username)
            .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;

        if !user.mfa_enabled {
            return Err(crate::Error::ValidationError(
                "MFA is not enabled".to_string(),
            ));
        }

        let secret = user
            .mfa_secret
            .ok_or_else(|| crate::Error::ValidationError("MFA not configured".to_string()))?;

        if !self.mfa_manager.verify_code(&secret, code) {
            return Err(crate::Error::AuthenticationError(
                "Invalid MFA code".to_string(),
            ));
        }

        drop(manager);
        let mut manager = self.auth_manager.write().await;
        let mut user = manager
            .get_user(username)
            .ok_or_else(|| crate::Error::ValidationError("User not found".to_string()))?;
        user.mfa_enabled = false;
        user.mfa_secret = None;
        manager.users.insert(username.to_string(), user);

        Ok(())
    }

    /// Report whether a user's roles require MFA for login.
    pub async fn mfa_required_for_login(&self, username: &str) -> bool {
        let manager = self.auth_manager.read().await;
        if let Some(user) = manager.get_user(username) {
            for role in &user.roles {
                if manager.config.mfa_required_for_roles.contains(role) {
                    return true;
                }
            }
        }
        false
    }
}
