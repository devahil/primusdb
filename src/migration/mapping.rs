//! Source-to-target field mapping configuration.
//!
//! Defines `MappingConfig` (loaded from a TOML file) and the `apply_mapping`
//! function that translates a `SourceSchema` into per-object `ObjectMapping`s.
//!
//! ```text
//! +-------------+     +-------------+
//! | MappingConfig|     | SourceSchema |
//! +------+------+     +------+------+
//!        |                    |
//!        +---------+----------+
//!                  |
//!                  v
//!        +-------------------+
//!        |  apply_mapping()  |
//!        +-------------------+
//!                  |
//!                  v
//!        +-------------------+
//!        | Vec<ObjectMapping> |
//!        +-------------------+
//! ```

use serde::{Deserialize, Serialize};

use crate::Result;

use super::source::{SourceObject, SourceSchema};
use super::target::{FieldMapping, ObjectMapping};

/// Configuration for a migration mapping file in TOML format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    /// Source database configuration.
    pub source: SourceConfig,
    /// Target PrimusDB configuration.
    pub target: TargetConfig,
    /// Object mappings defining source-to-target translations.
    pub objects: Vec<ObjectMapping>,
}

/// Source database connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Type of source database (e.g. "mysql", "postgres", "mongodb", "couchdb").
    pub r#type: String,
    /// Optional database name to scope the migration to.
    pub database: Option<String>,
}

/// Target PrimusDB configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Namespace to import data into.
    pub namespace: String,
    /// Default storage engine for objects that do not specify one.
    pub default_engine: String,
}

/// Parse a TOML mapping configuration string into a [`MappingConfig`].
pub fn parse_mapping(content: &str) -> Result<MappingConfig> {
    let config: MappingConfig =
        toml::from_str(content).map_err(|e| crate::Error::ConfigurationError(e.to_string()))?;
    Ok(config)
}

/// Validate a [`MappingConfig`] and return a list of warning or error messages.
///
/// Returns an empty vector if the configuration is valid.
pub fn validate_mapping(mapping: &MappingConfig) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    if mapping.source.r#type.is_empty() {
        warnings.push("source.type is required".to_string());
    }

    if mapping.target.namespace.is_empty() {
        warnings.push("target.namespace is required".to_string());
    }

    for (i, obj) in mapping.objects.iter().enumerate() {
        if obj.source.is_empty() {
            warnings.push(format!("object[{}].source is required", i));
        }
        if obj.target.is_empty() {
            warnings.push(format!("object[{}].target is required", i));
        }
        if obj.engine.is_empty() {
            warnings.push(format!("object[{}].engine is required", i));
        }
    }

    Ok(warnings)
}

/// Apply a [`MappingConfig`] to a source schema and produce concrete object mappings.
///
/// Returns an error if the mapping references objects not found in the schema.
pub fn apply_mapping(schema: &SourceSchema, mapping: &MappingConfig) -> Result<Vec<ObjectMapping>> {
    let mut result = Vec::new();

    // Build a lookup of all objects in the schema.
    let mut schema_objects: Vec<&SourceObject> = Vec::new();
    for db in &schema.databases {
        for obj in &db.objects {
            schema_objects.push(obj);
        }
    }

    for obj_mapping in &mapping.objects {
        // Verify the source object exists in the schema.
        let schema_obj = schema_objects
            .iter()
            .find(|o| {
                let qualified = format!("{}.{}", db_name(o, schema), o.name);
                qualified == obj_mapping.source || o.name == obj_mapping.source
            })
            .ok_or_else(|| {
                crate::Error::ValidationError(format!(
                    "Source object '{}' not found in schema",
                    obj_mapping.source
                ))
            })?;

        // If no field mappings were specified, create default 1:1 mappings.
        let field_mappings = if obj_mapping.field_mappings.is_empty() {
            schema_obj
                .columns
                .iter()
                .map(|c| FieldMapping {
                    source: c.name.clone(),
                    target: c.name.clone(),
                    type_override: None,
                })
                .collect()
        } else {
            obj_mapping.field_mappings.clone()
        };

        result.push(ObjectMapping {
            source: obj_mapping.source.clone(),
            target: obj_mapping.target.clone(),
            engine: if obj_mapping.engine.is_empty() {
                mapping.target.default_engine.clone()
            } else {
                obj_mapping.engine.clone()
            },
            primary_key: obj_mapping.primary_key.clone(),
            field_mappings,
        });
    }

    Ok(result)
}

fn db_name<'a>(obj: &'a SourceObject, schema: &'a SourceSchema) -> &'a str {
    for db in &schema.databases {
        if db.objects.iter().any(|o| std::ptr::eq(o, obj)) {
            return &db.name;
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::source::*;

    #[test]
    fn test_parse_mapping_valid() {
        let toml = r#"
[source]
type = "mysql"
database = "mydb"

[target]
namespace = "default"
default_engine = "relational"

[[objects]]
source = "mydb.users"
target = "users"
engine = "relational"
field_mappings = []
"#;
        let config = parse_mapping(toml).unwrap();
        assert_eq!(config.source.r#type, "mysql");
        assert_eq!(config.source.database, Some("mydb".into()));
        assert_eq!(config.target.namespace, "default");
        assert_eq!(config.target.default_engine, "relational");
        assert_eq!(config.objects.len(), 1);
        assert_eq!(config.objects[0].source, "mydb.users");
    }

    #[test]
    fn test_parse_mapping_invalid() {
        let result = parse_mapping("not valid toml {{");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mapping_valid() {
        let config = MappingConfig {
            source: SourceConfig {
                r#type: "mysql".into(),
                database: None,
            },
            target: TargetConfig {
                namespace: "default".into(),
                default_engine: "relational".into(),
            },
            objects: vec![ObjectMapping {
                source: "mydb.users".into(),
                target: "users".into(),
                engine: "relational".into(),
                primary_key: None,
                field_mappings: vec![],
            }],
        };
        let warnings = validate_mapping(&config).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_mapping_missing_fields() {
        let config = MappingConfig {
            source: SourceConfig {
                r#type: "".into(),
                database: None,
            },
            target: TargetConfig {
                namespace: "".into(),
                default_engine: "relational".into(),
            },
            objects: vec![ObjectMapping {
                source: "".into(),
                target: "".into(),
                engine: "".into(),
                primary_key: None,
                field_mappings: vec![],
            }],
        };
        let warnings = validate_mapping(&config).unwrap();
        assert!(warnings.iter().any(|w| w.contains("source.type")));
        assert!(warnings.iter().any(|w| w.contains("target.namespace")));
        assert!(warnings.iter().any(|w| w.contains("object[0].source")));
        assert!(warnings.iter().any(|w| w.contains("object[0].target")));
        assert!(warnings.iter().any(|w| w.contains("object[0].engine")));
    }

    #[test]
    fn test_apply_mapping_simple() {
        let schema = SourceSchema {
            databases: vec![SourceDatabase {
                name: "mydb".into(),
                objects: vec![SourceObject {
                    name: "users".into(),
                    object_type: "table".into(),
                    columns: vec![SourceColumn {
                        name: "id".into(),
                        data_type: "int".into(),
                        nullable: false,
                        is_primary_key: true,
                        max_length: None,
                    }],
                    row_estimate: None,
                    primary_key: vec!["id".into()],
                }],
            }],
        };
        let mapping = MappingConfig {
            source: SourceConfig {
                r#type: "mysql".into(),
                database: None,
            },
            target: TargetConfig {
                namespace: "ns".into(),
                default_engine: "relational".into(),
            },
            objects: vec![ObjectMapping {
                source: "mydb.users".into(),
                target: "users_copy".into(),
                engine: "".into(),
                primary_key: Some("id".into()),
                field_mappings: vec![],
            }],
        };
        let result = apply_mapping(&schema, &mapping).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].target, "users_copy");
        assert_eq!(result[0].engine, "relational");
        assert_eq!(result[0].field_mappings.len(), 1);
        assert_eq!(result[0].field_mappings[0].source, "id");
    }

    #[test]
    fn test_apply_mapping_object_not_found() {
        let schema = SourceSchema { databases: vec![] };
        let mapping = MappingConfig {
            source: SourceConfig {
                r#type: "mysql".into(),
                database: None,
            },
            target: TargetConfig {
                namespace: "ns".into(),
                default_engine: "relational".into(),
            },
            objects: vec![ObjectMapping {
                source: "nonexistent.table".into(),
                target: "t".into(),
                engine: "relational".into(),
                primary_key: None,
                field_mappings: vec![],
            }],
        };
        let result = apply_mapping(&schema, &mapping);
        assert!(result.is_err());
    }
}
