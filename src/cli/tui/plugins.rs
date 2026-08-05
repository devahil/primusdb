use crate::cli::tui::app::NavSection;
use std::collections::HashMap;

pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

pub struct PluginRegistry {
    plugins: Vec<PluginInfo>,
    _workspaces: HashMap<NavSection, String>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            _workspaces: HashMap::new(),
        }
    }

    pub fn register(&mut self, info: PluginInfo) {
        self.plugins.push(info);
    }

    pub fn plugins(&self) -> &[PluginInfo] {
        &self.plugins
    }

    pub fn doctor_report(&self) -> Vec<String> {
        let mut r = vec![format!("Registered plugins: {}", self.plugins.len())];
        for p in &self.plugins {
            r.push(format!("  {} v{} by {}", p.name, p.version, p.author));
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plugin(name: &str) -> PluginInfo {
        PluginInfo {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            description: "A test plugin".to_string(),
        }
    }

    #[test]
    fn test_plugin_registry_new() {
        let reg = PluginRegistry::new();
        assert!(reg.plugins().is_empty());
    }

    #[test]
    fn test_plugin_registry_default() {
        let reg = PluginRegistry::default();
        assert!(reg.plugins().is_empty());
    }

    #[test]
    fn test_register_plugin() {
        let mut reg = PluginRegistry::new();
        reg.register(sample_plugin("my-plugin"));
        assert_eq!(reg.plugins().len(), 1);
        assert_eq!(reg.plugins()[0].name, "my-plugin");
    }

    #[test]
    fn test_register_multiple_plugins() {
        let mut reg = PluginRegistry::new();
        reg.register(sample_plugin("alpha"));
        reg.register(sample_plugin("beta"));
        reg.register(sample_plugin("gamma"));
        assert_eq!(reg.plugins().len(), 3);
    }

    #[test]
    fn test_doctor_report_empty() {
        let reg = PluginRegistry::new();
        let report = reg.doctor_report();
        assert_eq!(report.len(), 1);
        assert!(report[0].contains("0"));
    }

    #[test]
    fn test_doctor_report_with_plugins() {
        let mut reg = PluginRegistry::new();
        reg.register(sample_plugin("my-plugin"));
        let report = reg.doctor_report();
        assert_eq!(report.len(), 2);
        assert!(report[0].contains("1"));
        assert!(report[1].contains("my-plugin"));
        assert!(report[1].contains("1.0.0"));
        assert!(report[1].contains("test"));
    }

    #[test]
    fn test_plugin_info_fields() {
        let p = PluginInfo {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            author: "dev".to_string(),
            description: "desc".to_string(),
        };
        assert_eq!(p.name, "test");
        assert_eq!(p.version, "0.1.0");
        assert_eq!(p.author, "dev");
        assert_eq!(p.description, "desc");
    }
}
