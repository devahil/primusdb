pub mod resolver;
pub mod storage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sled::Db;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::StorageType;

pub fn validate_namespace_component(component: &str) -> bool {
    if component.is_empty() || component.len() > 64 {
        return false;
    }
    let bytes = component.as_bytes();
    if !bytes[0].is_ascii_alphabetic() && bytes[0] != b'_' {
        return false;
    }
    for &b in &bytes[1..] {
        if !b.is_ascii_alphanumeric() && b != b'_' {
            return false;
        }
    }
    true
}

pub fn validate_namespace_path(path: &str) -> crate::Result<()> {
    if path.is_empty() {
        return Err(crate::Error::ValidationError(
            "Namespace path cannot be empty".to_string(),
        ));
    }
    if path.len() > 1024 {
        return Err(crate::Error::ValidationError(
            "Namespace path too long (max 1024 chars)".to_string(),
        ));
    }
    for component in path.split('.') {
        if !validate_namespace_component(component) {
            return Err(crate::Error::ValidationError(format!(
                "Invalid namespace component: '{}'. Must match ^[a-zA-Z_][a-zA-Z0-9_]{{0,63}}$",
                component
            )));
        }
    }
    Ok(())
}

pub fn compute_physical_name(namespace_path: &str, resource_name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(namespace_path.as_bytes());
    let hash = hasher.finalize();
    let short_hash = hex::encode(&hash[..6]);
    format!("ns_{}__{}", short_hash, resource_name)
}

pub fn parent_path(path: &str) -> Option<String> {
    let dot = path.rfind('.')?;
    Some(path[..dot].to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceConfig {
    pub enabled: bool,
    pub default_namespace: String,
    pub strict_isolation: bool,
    pub allow_cross_namespace_queries: bool,
    pub cache_size: usize,
    pub max_depth: u32,
    pub allow_legacy_without_namespace: bool,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_namespace: "root.default".to_string(),
            strict_isolation: true,
            allow_cross_namespace_queries: false,
            cache_size: 10_000,
            max_depth: 16,
            allow_legacy_without_namespace: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum InheritanceMode {
    DenyOverride,
    ExplicitOnly,
    #[default]
    AllowOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespacePolicies {
    pub inheritance_mode: InheritanceMode,
    pub max_depth: u32,
    pub max_resources: u32,
    pub max_users: u32,
    pub max_storage_bytes: u64,
    pub allowed_storage_types: Vec<StorageType>,
    pub allowed_actions: Vec<String>,
    pub include_children_in_quota: bool,
    pub retention_days: Option<u32>,
    pub custom_policies: HashMap<String, serde_json::Value>,
}

impl Default for NamespacePolicies {
    fn default() -> Self {
        Self {
            inheritance_mode: InheritanceMode::AllowOverride,
            max_depth: 16,
            max_resources: 1000,
            max_users: 500,
            max_storage_bytes: 10 * 1024 * 1024 * 1024,
            allowed_storage_types: vec![
                StorageType::Columnar,
                StorageType::Vector,
                StorageType::Document,
                StorageType::Relational,
                StorageType::KeyValue,
            ],
            allowed_actions: vec![],
            include_children_in_quota: true,
            retention_days: None,
            custom_policies: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum NamespacePermission {
    Create,
    Read,
    Update,
    Delete,
    AttachResource,
    DetachResource,
    ManageUsers,
    ManageRoles,
    ManagePolicies,
    CrossNamespaceRead,
    CrossNamespaceWrite,
    FullAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub id: String,
    pub path: String,
    pub parent_path: Option<String>,
    pub description: String,
    pub policies: NamespacePolicies,
    pub segment_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceResource {
    pub id: String,
    pub namespace_id: String,
    pub storage_type: StorageType,
    pub resource_name: String,
    pub physical_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceUserBinding {
    pub namespace_id: String,
    pub user_id: String,
    pub role_id: String,
    pub granted_at: DateTime<Utc>,
    pub granted_by: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceRole {
    pub id: String,
    pub namespace_id: String,
    pub name: String,
    pub description: String,
    pub permissions: Vec<NamespacePermission>,
    pub inheritable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceContext {
    pub current_namespace: Option<String>,
    pub original_namespace: Option<String>,
    pub resolved: bool,
}

impl Default for NamespaceContext {
    fn default() -> Self {
        Self {
            current_namespace: Some("root.default".to_string()),
            original_namespace: None,
            resolved: true,
        }
    }
}

pub struct NamespaceController {
    config: NamespaceConfig,
    db: Arc<Db>,
    cache: RwLock<HashMap<String, Namespace>>,
    path_index: RwLock<HashMap<String, String>>,
}

impl NamespaceController {
    pub fn new(config: &crate::PrimusDBConfig) -> crate::Result<Self> {
        let ns_config = config.namespaces.clone();
        let path = format!("{}/namespace", config.storage.data_dir);
        std::fs::create_dir_all(&path)?;
        let db = Arc::new(sled::open(&path)?);
        Ok(Self {
            config: ns_config,
            db,
            cache: RwLock::new(HashMap::new()),
            path_index: RwLock::new(HashMap::new()),
        })
    }

    pub fn config(&self) -> &NamespaceConfig {
        &self.config
    }

    pub fn db(&self) -> &Arc<Db> {
        &self.db
    }

    pub fn init(&self) -> crate::Result<()> {
        self.ensure_system_namespaces()?;
        self.rebuild_cache()?;
        Ok(())
    }

    fn ensure_system_namespaces(&self) -> crate::Result<()> {
        if self.get_by_path("root")?.is_none() {
            self.create_raw(Namespace {
                id: uuid::Uuid::new_v4().to_string(),
                path: "root".to_string(),
                parent_path: None,
                description: "Root namespace - system reserved".to_string(),
                policies: NamespacePolicies {
                    max_depth: 32,
                    ..Default::default()
                },
                segment_id: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                is_active: true,
                metadata: HashMap::new(),
            })?;
        }

        if self.get_by_path(&self.config.default_namespace)?.is_none() {
            let default_path = &self.config.default_namespace;
            let parent = parent_path(default_path);
            if let Some(ref p) = parent {
                if self.get_by_path(p)?.is_none() {
                    self.create_raw(Namespace {
                        id: uuid::Uuid::new_v4().to_string(),
                        path: p.clone(),
                        parent_path: parent_path(p),
                        description: format!("Parent namespace for '{}'", default_path),
                        policies: NamespacePolicies::default(),
                        segment_id: None,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        is_active: true,
                        metadata: HashMap::new(),
                    })?;
                }
            }

            self.create_raw(Namespace {
                id: uuid::Uuid::new_v4().to_string(),
                path: default_path.clone(),
                parent_path: parent,
                description: "Default namespace for backward compatibility".to_string(),
                policies: NamespacePolicies::default(),
                segment_id: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                is_active: true,
                metadata: HashMap::new(),
            })?;
        }

        Ok(())
    }

    fn rebuild_cache(&self) -> crate::Result<()> {
        let mut cache = self.cache.write().unwrap();
        let mut path_index = self.path_index.write().unwrap();
        cache.clear();
        path_index.clear();

        let mut iter = self.db.open_tree("namespaces")?.iter();
        while let Some(Ok((key, value))) = iter.next() {
            if let Ok(key_str) = String::from_utf8(key.to_vec()) {
                if let Some(stripped) = key_str.strip_prefix("namespace:") {
                    if let Ok(ns) = bincode::deserialize::<Namespace>(&value) {
                        let path = ns.path.clone();
                        cache.insert(ns.id.clone(), ns);
                        path_index.insert(path, stripped.to_string());
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_by_path(&self, path: &str) -> crate::Result<Option<Namespace>> {
        {
            let path_index = self.path_index.read().unwrap();
            if let Some(id) = path_index.get(path) {
                let cache = self.cache.read().unwrap();
                if let Some(ns) = cache.get(id) {
                    return Ok(Some(ns.clone()));
                }
            }
        }

        let tree = self.db.open_tree("namespaces")?;
        if let Some(value) = tree.get(format!("path:{}", path).as_bytes())? {
            if let Ok(ns) = bincode::deserialize::<Namespace>(&value) {
                let mut cache = self.cache.write().unwrap();
                let mut path_index = self.path_index.write().unwrap();
                cache.insert(ns.id.clone(), ns.clone());
                path_index.insert(path.to_string(), ns.id.clone());
                return Ok(Some(ns));
            }
        }

        Ok(None)
    }

    pub fn get_by_id(&self, id: &str) -> crate::Result<Option<Namespace>> {
        {
            let cache = self.cache.read().unwrap();
            if let Some(ns) = cache.get(id) {
                return Ok(Some(ns.clone()));
            }
        }

        let tree = self.db.open_tree("namespaces")?;
        if let Some(value) = tree.get(format!("namespace:{}", id).as_bytes())? {
            if let Ok(ns) = bincode::deserialize::<Namespace>(&value) {
                let mut cache = self.cache.write().unwrap();
                let mut path_index = self.path_index.write().unwrap();
                cache.insert(ns.id.clone(), ns.clone());
                path_index.insert(ns.path.clone(), ns.id.clone());
                return Ok(Some(ns));
            }
        }

        Ok(None)
    }

    pub fn create(
        &self,
        path: &str,
        description: &str,
        policies: Option<NamespacePolicies>,
        segment_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> crate::Result<Namespace> {
        if !self.config.enabled {
            return Err(crate::Error::ValidationError(
                "Namespaces are not enabled".to_string(),
            ));
        }

        validate_namespace_path(path)?;
        let depth = path.split('.').count() as u32;
        if depth > self.config.max_depth {
            return Err(crate::Error::ValidationError(format!(
                "Namespace path too deep (max depth: {})",
                self.config.max_depth
            )));
        }

        if self.get_by_path(path)?.is_some() {
            return Err(crate::Error::ValidationError(format!(
                "Namespace '{}' already exists",
                path
            )));
        }

        let parent = parent_path(path);
        if let Some(ref p) = parent {
            if self.get_by_path(p)?.is_none() {
                return Err(crate::Error::ValidationError(format!(
                    "Parent namespace '{}' does not exist",
                    p
                )));
            }
        }

        let now = Utc::now();
        let ns = Namespace {
            id: uuid::Uuid::new_v4().to_string(),
            path: path.to_string(),
            parent_path: parent,
            description: description.to_string(),
            policies: policies.unwrap_or_default(),
            segment_id,
            created_at: now,
            updated_at: now,
            is_active: true,
            metadata,
        };

        self.persist_namespace(&ns)?;
        self.update_cache(ns.clone());

        Ok(ns)
    }

    fn create_raw(&self, ns: Namespace) -> crate::Result<()> {
        self.persist_namespace(&ns)?;
        self.update_cache(ns);
        Ok(())
    }

    fn persist_namespace(&self, ns: &Namespace) -> crate::Result<()> {
        let tree = self.db.open_tree("namespaces")?;
        let value = bincode::serialize(ns)?;
        tree.insert(format!("namespace:{}", ns.id).as_bytes(), value)?;
        tree.insert(
            format!("path:{}", ns.path).as_bytes(),
            bincode::serialize(ns)?,
        )?;
        Ok(())
    }

    fn update_cache(&self, ns: Namespace) {
        let mut cache = self.cache.write().unwrap();
        let mut path_index = self.path_index.write().unwrap();
        cache.insert(ns.id.clone(), ns.clone());
        path_index.insert(ns.path.clone(), ns.id);
    }

    pub fn update(&self, path: &str, updates: NamespaceUpdate) -> crate::Result<Namespace> {
        let ns = self.get_by_path(path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", path))
        })?;

        let updated = Namespace {
            description: updates.description.unwrap_or(ns.description),
            policies: updates.policies.unwrap_or(ns.policies),
            segment_id: updates.segment_id.or(ns.segment_id),
            is_active: updates.is_active.unwrap_or(ns.is_active),
            metadata: {
                let mut m = ns.metadata.clone();
                if let Some(meta) = updates.metadata {
                    for (k, v) in meta {
                        if v.is_null() {
                            m.remove(&k);
                        } else if let Some(s) = v.as_str() {
                            m.insert(k, s.to_string());
                        }
                    }
                }
                m
            },
            updated_at: Utc::now(),
            ..ns
        };

        self.persist_namespace(&updated)?;
        self.update_cache(updated.clone());
        Ok(updated)
    }

    pub fn delete(&self, path: &str) -> crate::Result<()> {
        if path == "root" || path == self.config.default_namespace {
            return Err(crate::Error::ValidationError(format!(
                "Cannot delete system namespace '{}'",
                path
            )));
        }

        let ns = self.get_by_path(path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", path))
        })?;

        let children = self.list_children(path)?;
        if !children.is_empty() {
            return Err(crate::Error::ValidationError(format!(
                "Namespace '{}' has {} child namespace(s). Delete them first or use cascade.",
                path,
                children.len()
            )));
        }

        let tree = self.db.open_tree("namespaces")?;
        tree.remove(format!("namespace:{}", ns.id).as_bytes())?;
        tree.remove(format!("path:{}", ns.path).as_bytes())?;

        let mut cache = self.cache.write().unwrap();
        let mut path_index = self.path_index.write().unwrap();
        cache.remove(&ns.id);
        path_index.remove(path);

        Ok(())
    }

    pub fn list_all(&self) -> crate::Result<Vec<Namespace>> {
        let cache = self.cache.read().unwrap();
        let mut namespaces: Vec<Namespace> = cache.values().cloned().collect();
        namespaces.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(namespaces)
    }

    pub fn list_children(&self, parent_path: &str) -> crate::Result<Vec<Namespace>> {
        let cache = self.cache.read().unwrap();
        let prefix = format!("{}.", parent_path);
        let mut children: Vec<Namespace> = cache
            .values()
            .filter(|ns| {
                ns.path.starts_with(&prefix) && ns.parent_path.as_deref() == Some(parent_path)
            })
            .cloned()
            .collect();
        children.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(children)
    }

    pub fn resolve(&self, namespace_hint: Option<&str>) -> crate::Result<String> {
        match namespace_hint {
            Some(path) if !path.is_empty() => {
                validate_namespace_path(path)?;
                if self.get_by_path(path)?.is_some() {
                    Ok(path.to_string())
                } else {
                    Ok(self.config.default_namespace.clone())
                }
            }
            _ => Ok(self.config.default_namespace.clone()),
        }
    }

    pub fn attach_resource(
        &self,
        namespace_path: &str,
        storage_type: StorageType,
        resource_name: &str,
    ) -> crate::Result<NamespaceResource> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let resources = self.list_resources(&ns.id)?;
        if resources.len() as u32 >= ns.policies.max_resources {
            return Err(crate::Error::ValidationError(format!(
                "Namespace '{}' has reached max resources ({})",
                namespace_path, ns.policies.max_resources
            )));
        }

        if !ns.policies.allowed_storage_types.contains(&storage_type) {
            return Err(crate::Error::ValidationError(format!(
                "Storage type '{:?}' not allowed in namespace '{}'",
                storage_type, namespace_path
            )));
        }

        let resource = NamespaceResource {
            id: uuid::Uuid::new_v4().to_string(),
            namespace_id: ns.id.clone(),
            storage_type,
            resource_name: resource_name.to_string(),
            physical_name: compute_physical_name(namespace_path, resource_name),
            created_at: Utc::now(),
        };

        let tree = self.db.open_tree("resources")?;
        let key = format!("resource:{}:{:?}:{}", ns.id, storage_type, resource_name);
        tree.insert(key.as_bytes(), bincode::serialize(&resource)?)?;

        Ok(resource)
    }

    pub fn detach_resource(
        &self,
        namespace_path: &str,
        storage_type: StorageType,
        resource_name: &str,
    ) -> crate::Result<()> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let tree = self.db.open_tree("resources")?;
        let key = format!("resource:{}:{:?}:{}", ns.id, storage_type, resource_name);
        tree.remove(key.as_bytes())?;
        Ok(())
    }

    pub fn list_resources(&self, namespace_id: &str) -> crate::Result<Vec<NamespaceResource>> {
        let tree = self.db.open_tree("resources")?;
        let prefix = format!("resource:{}:", namespace_id);
        let mut resources = Vec::new();
        let mut iter = tree.scan_prefix(prefix.as_bytes());
        while let Some(Ok((_, value))) = iter.next() {
            if let Ok(r) = bincode::deserialize::<NamespaceResource>(&value) {
                resources.push(r);
            }
        }
        resources.sort_by(|a, b| a.resource_name.cmp(&b.resource_name));
        Ok(resources)
    }

    pub fn resolve_physical_name(
        &self,
        namespace_path: &str,
        storage_type: StorageType,
        resource_name: &str,
    ) -> crate::Result<String> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let tree = self.db.open_tree("resources")?;
        let key = format!("resource:{}:{:?}:{}", ns.id, storage_type, resource_name);
        if let Some(value) = tree.get(key.as_bytes())? {
            let resource = bincode::deserialize::<NamespaceResource>(&value)?;
            Ok(resource.physical_name)
        } else {
            Err(crate::Error::ValidationError(format!(
                "Resource '{}' not attached to namespace '{}' for storage type {:?}",
                resource_name, namespace_path, storage_type
            )))
        }
    }

    pub fn add_user_binding(
        &self,
        namespace_path: &str,
        user_id: &str,
        role_id: &str,
        granted_by: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> crate::Result<NamespaceUserBinding> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let binding = NamespaceUserBinding {
            namespace_id: ns.id.clone(),
            user_id: user_id.to_string(),
            role_id: role_id.to_string(),
            granted_at: Utc::now(),
            granted_by: granted_by.to_string(),
            expires_at,
        };

        let tree = self.db.open_tree("user_bindings")?;
        let key = format!("user_binding:{}:{}", ns.id, user_id);
        tree.insert(key.as_bytes(), bincode::serialize(&binding)?)?;

        Ok(binding)
    }

    pub fn remove_user_binding(&self, namespace_path: &str, user_id: &str) -> crate::Result<()> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let tree = self.db.open_tree("user_bindings")?;
        let key = format!("user_binding:{}:{}", ns.id, user_id);
        tree.remove(key.as_bytes())?;
        Ok(())
    }

    pub fn list_user_bindings(
        &self,
        namespace_id: &str,
    ) -> crate::Result<Vec<NamespaceUserBinding>> {
        let tree = self.db.open_tree("user_bindings")?;
        let prefix = format!("user_binding:{}:", namespace_id);
        let mut bindings = Vec::new();
        let mut iter = tree.scan_prefix(prefix.as_bytes());
        while let Some(Ok((_, value))) = iter.next() {
            if let Ok(b) = bincode::deserialize::<NamespaceUserBinding>(&value) {
                bindings.push(b);
            }
        }
        Ok(bindings)
    }

    pub fn add_role(
        &self,
        namespace_path: &str,
        name: &str,
        description: &str,
        permissions: Vec<NamespacePermission>,
        inheritable: bool,
    ) -> crate::Result<NamespaceRole> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let role = NamespaceRole {
            id: uuid::Uuid::new_v4().to_string(),
            namespace_id: ns.id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            permissions,
            inheritable,
        };

        let tree = self.db.open_tree("roles")?;
        let key = format!("role:{}:{}", ns.id, role.id);
        tree.insert(key.as_bytes(), bincode::serialize(&role)?)?;

        Ok(role)
    }

    pub fn remove_role(&self, namespace_path: &str, role_id: &str) -> crate::Result<()> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let tree = self.db.open_tree("roles")?;
        let key = format!("role:{}:{}", ns.id, role_id);
        tree.remove(key.as_bytes())?;
        Ok(())
    }

    pub fn list_roles(&self, namespace_id: &str) -> crate::Result<Vec<NamespaceRole>> {
        let tree = self.db.open_tree("roles")?;
        let prefix = format!("role:{}:", namespace_id);
        let mut roles = Vec::new();
        let mut iter = tree.scan_prefix(prefix.as_bytes());
        while let Some(Ok((_, value))) = iter.next() {
            if let Ok(r) = bincode::deserialize::<NamespaceRole>(&value) {
                roles.push(r);
            }
        }
        Ok(roles)
    }

    pub fn effective_policy(&self, namespace_path: &str) -> crate::Result<NamespacePolicies> {
        let ns = self.get_by_path(namespace_path)?.ok_or_else(|| {
            crate::Error::ValidationError(format!("Namespace '{}' not found", namespace_path))
        })?;

        let mut merged = ns.policies.clone();

        let mut current = ns.parent_path.clone();
        while let Some(ref parent_path_str) = current {
            if let Some(parent_ns) = self.get_by_path(parent_path_str)? {
                let parent_policy = parent_ns.policies;
                match merged.inheritance_mode {
                    InheritanceMode::DenyOverride => {
                        if parent_policy.allowed_storage_types.is_empty() {
                            merged.allowed_storage_types.clear();
                        }
                        merged.max_depth = merged.max_depth.min(parent_policy.max_depth);
                        merged.max_resources =
                            merged.max_resources.min(parent_policy.max_resources);
                        merged.max_users = merged.max_users.min(parent_policy.max_users);
                        merged.max_storage_bytes = merged
                            .max_storage_bytes
                            .min(parent_policy.max_storage_bytes);
                    }
                    InheritanceMode::ExplicitOnly => {}
                    InheritanceMode::AllowOverride => {
                        if merged.max_depth == 0 {
                            merged.max_depth = parent_policy.max_depth;
                        }
                        if merged.max_resources == 0 {
                            merged.max_resources = parent_policy.max_resources;
                        }
                        if merged.max_users == 0 {
                            merged.max_users = parent_policy.max_users;
                        }
                        if merged.max_storage_bytes == 0 {
                            merged.max_storage_bytes = parent_policy.max_storage_bytes;
                        }
                    }
                }
            }
            current = parent_path(parent_path_str);
        }

        Ok(merged)
    }

    pub fn invalidate_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        let mut path_index = self.path_index.write().unwrap();
        cache.clear();
        path_index.clear();
    }

    pub fn depth(&self, path: &str) -> u32 {
        path.split('.').count() as u32
    }
}

#[derive(Debug, Default)]
pub struct NamespaceUpdate {
    pub description: Option<String>,
    pub policies: Option<NamespacePolicies>,
    pub segment_id: Option<String>,
    pub is_active: Option<bool>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}
