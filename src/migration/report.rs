//! Migration reporting — generates human-readable reports in Markdown.
//!
//! The report aggregates timing, object/row counts, errors, warnings, and
//! optional validation results. All sensitive credentials are masked.
//!
//! ```text
//! +-------------+
//! | MigrationReport |
//! +------+------+
//!        |
//!        v
//! +-------------+
//! | render_markdown() |
//! +------+------+
//!        |
//!        v
//! +-------------+
//! | Markdown String |
//! +-------------+
//! ```

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub objects_checked: u64,
    pub rows_matched: u64,
    pub checksums_matched: u64,
    pub mismatches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub source_type: String,
    pub source_url_masked: String,
    pub target_url: String,
    pub namespace: String,
    pub mode: String,
    pub started_at: String,
    pub duration_ms: u64,
    pub objects_total: u64,
    pub objects_imported: u64,
    pub rows_total: u64,
    pub rows_imported: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub validation: Option<ValidationReport>,
}

impl MigrationReport {
    pub fn mask_url(url: &str) -> String {
        let parts: Vec<&str> = url.splitn(3, "://").collect();
        if parts.len() != 2 {
            return url.to_string();
        }
        let scheme = parts[0];
        let rest = parts[1];
        if let Some(at_pos) = rest.find('@') {
            format!("{}://*****@{}", scheme, &rest[at_pos + 1..])
        } else {
            url.to_string()
        }
    }

    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Migration Report\n\n");
        out.push_str(&format!(
            "- **Source**: {} ({})\n",
            self.source_type, self.source_url_masked
        ));
        out.push_str(&format!("- **Target**: {}\n", self.target_url));
        out.push_str(&format!("- **Namespace**: {}\n", self.namespace));
        out.push_str(&format!("- **Mode**: {}\n", self.mode));
        out.push_str(&format!("- **Started**: {}\n", self.started_at));
        out.push_str(&format!("- **Duration**: {}ms\n", self.duration_ms));
        out.push_str(&format!(
            "- **Objects**: {}/{}\n",
            self.objects_imported, self.objects_total
        ));
        out.push_str(&format!(
            "- **Rows**: {}/{}\n",
            self.rows_imported, self.rows_total
        ));

        if !self.errors.is_empty() {
            out.push_str("\n## Errors\n\n");
            for e in &self.errors {
                out.push_str(&format!("- {}\n", e));
            }
        }

        if !self.warnings.is_empty() {
            out.push_str("\n## Warnings\n\n");
            for w in &self.warnings {
                out.push_str(&format!("- {}\n", w));
            }
        }

        if let Some(ref val) = self.validation {
            out.push_str("\n## Validation\n\n");
            out.push_str(&format!("- Objects checked: {}\n", val.objects_checked));
            out.push_str(&format!("- Rows matched: {}\n", val.rows_matched));
            out.push_str(&format!("- Checksums matched: {}\n", val.checksums_matched));
            if val.mismatches.is_empty() {
                out.push_str("- Result: All checks passed\n");
            } else {
                for m in &val.mismatches {
                    out.push_str(&format!("- Mismatch: {}\n", m));
                }
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_url_no_credentials() {
        let url = "http://localhost:8080";
        assert_eq!(MigrationReport::mask_url(url), url);
    }

    #[test]
    fn test_mask_url_with_credentials() {
        let url = "mysql://user:password@host:3306/db";
        let masked = MigrationReport::mask_url(url);
        assert!(masked.contains("*****"));
        assert!(!masked.contains("password"));
        assert!(!masked.contains("user"));
        assert!(masked.contains("host:3306/db"));
    }

    #[test]
    fn test_mask_url_no_scheme() {
        let url = "localhost:8080";
        assert_eq!(MigrationReport::mask_url(url), url);
    }

    #[test]
    fn test_render_markdown_basic() {
        let report = MigrationReport {
            source_type: "mysql".into(),
            source_url_masked: "mysql://*****@host/db".into(),
            target_url: "http://localhost:8080".into(),
            namespace: "default".into(),
            mode: "copy".into(),
            started_at: "2025-01-01T00:00:00Z".into(),
            duration_ms: 1500,
            objects_total: 5,
            objects_imported: 5,
            rows_total: 1000,
            rows_imported: 1000,
            errors: vec![],
            warnings: vec![],
            validation: None,
        };
        let md = report.render_markdown();
        assert!(md.contains("Migration Report"));
        assert!(md.contains("mysql"));
        assert!(md.contains("1000"));
        assert!(md.contains("5/5"));
    }

    #[test]
    fn test_render_markdown_with_errors() {
        let report = MigrationReport {
            source_type: "postgres".into(),
            source_url_masked: "postgres://*****@host/db".into(),
            target_url: "http://localhost:8080".into(),
            namespace: "default".into(),
            mode: "data-only".into(),
            started_at: "2025-01-01T00:00:00Z".into(),
            duration_ms: 500,
            objects_total: 2,
            objects_imported: 1,
            rows_total: 200,
            rows_imported: 100,
            errors: vec!["Failed to import table X".into()],
            warnings: vec!["Column Y omitted".into()],
            validation: None,
        };
        let md = report.render_markdown();
        assert!(md.contains("Errors"));
        assert!(md.contains("Failed to import table X"));
        assert!(md.contains("Warnings"));
        assert!(md.contains("Column Y omitted"));
    }

    #[test]
    fn test_render_markdown_with_validation_ok() {
        let report = MigrationReport {
            source_type: "mysql".into(),
            source_url_masked: "mysql://*****@host/db".into(),
            target_url: "http://localhost:8080".into(),
            namespace: "default".into(),
            mode: "copy".into(),
            started_at: "2025-01-01T00:00:00Z".into(),
            duration_ms: 1000,
            objects_total: 3,
            objects_imported: 3,
            rows_total: 500,
            rows_imported: 500,
            errors: vec![],
            warnings: vec![],
            validation: Some(ValidationReport {
                objects_checked: 3,
                rows_matched: 500,
                checksums_matched: 3,
                mismatches: vec![],
            }),
        };
        let md = report.render_markdown();
        assert!(md.contains("Validation"));
        assert!(md.contains("All checks passed"));
    }

    #[test]
    fn test_render_markdown_with_validation_mismatches() {
        let report = MigrationReport {
            source_type: "mysql".into(),
            source_url_masked: "mysql://*****@host/db".into(),
            target_url: "http://localhost:8080".into(),
            namespace: "default".into(),
            mode: "copy".into(),
            started_at: "2025-01-01T00:00:00Z".into(),
            duration_ms: 2000,
            objects_total: 1,
            objects_imported: 1,
            rows_total: 100,
            rows_imported: 100,
            errors: vec![],
            warnings: vec![],
            validation: Some(ValidationReport {
                objects_checked: 1,
                rows_matched: 50,
                checksums_matched: 0,
                mismatches: vec!["Row count mismatch".into()],
            }),
        };
        let md = report.render_markdown();
        assert!(md.contains("Validation"));
        assert!(md.contains("Mismatch"));
        assert!(md.contains("Row count mismatch"));
    }
}
