use super::NamespaceController;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct NamespaceResolver {
    controller: Arc<NamespaceController>,
    cache: RwLock<HashMap<String, String>>,
}

impl NamespaceResolver {
    pub fn new(controller: Arc<NamespaceController>) -> Self {
        Self {
            controller,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn resolve(
        &self,
        header: Option<&str>,
        token_namespace: Option<&str>,
        default: Option<&str>,
    ) -> String {
        let candidates = [header, token_namespace, default, Some("root.default")];
        for candidate in candidates.iter().flatten() {
            if candidate.is_empty() {
                continue;
            }
            if self.resolve_cached(candidate) {
                return candidate.to_string();
            }
        }
        "root.default".to_string()
    }

    fn resolve_cached(&self, path: &str) -> bool {
        {
            let cache = self.cache.read().unwrap();
            if let Some(result) = cache.get(path) {
                return result == "valid";
            }
        }
        let exists = self.controller.get_by_path(path).unwrap_or(None).is_some();
        let mut cache = self.cache.write().unwrap();
        cache.insert(path.to_string(), if exists { "valid" } else { "invalid" }.to_string());
        exists
    }

    pub fn invalidate(&self, path: Option<&str>) {
        let mut cache = self.cache.write().unwrap();
        match path {
            Some(p) => {
                cache.remove(p);
            }
            None => cache.clear(),
        }
    }
}
