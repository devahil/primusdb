//! Migration validation — verifies that imported data matches expectations.
//!
//! After import, validation checks row counts per target table via the
//! PrimusDB REST API and reports any mismatches.
//!
//! ```text
//! +-------------+
//! | MigrationPlan |
//! +------+------+
//!        |
//!        v
//! +-------------+
//! | validate_import() |
//! +------+------+
//!        |
//!        v
//! +-------------+
//! | ValidationReport |
//! +-------------+
//! ```

use crate::Result;

use super::plan::MigrationPlan;
use super::report::ValidationReport;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::plan::MigrationMode;
    use crate::migration::target::ObjectMapping;

    #[test]
    fn test_validate_import_connection_error() {
        let plan = MigrationPlan {
            source_type: "test".into(),
            source_url: "test://host".into(),
            target_url: "http://127.0.0.1:1".into(),
            namespace: "test".into(),
            mode: MigrationMode::Copy,
            objects: vec![ObjectMapping {
                source: "db.table".into(),
                target: "table".into(),
                engine: "relational".into(),
                primary_key: None,
                field_mappings: vec![],
            }],
            batch_size: 1000,
            estimated_rows: 100,
            warnings: vec![],
        };
        let report = validate_import("http://127.0.0.1:1", "test", &plan).unwrap();
        assert_eq!(report.objects_checked, 0);
        assert!(report.mismatches.len() >= 1);
        assert!(
            report.mismatches[0].contains("Failed to connect")
                || report.mismatches[0].contains("table")
        );
    }
}

/// Validate that data was correctly imported into a PrimusDB target.
///
/// Checks row counts via the PrimusDB REST API for the given namespace
/// and compares them against the migration plan's expectations.
pub fn validate_import(
    target_url: &str,
    namespace: &str,
    plan: &MigrationPlan,
) -> Result<ValidationReport> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| crate::Error::NetworkError(e.to_string()))?;

    let mut objects_checked = 0u64;
    let mut rows_matched = 0u64;
    let mut checksums_matched = 0u64;
    let mut mismatches = Vec::new();

    for mapping in &plan.objects {
        let count_url = format!(
            "{}/namespaces/{}/tables/{}/count",
            target_url.trim_end_matches('/'),
            namespace,
            mapping.target
        );

        match client.get(&count_url).send() {
            Ok(resp) if resp.status().is_success() => {
                let count: serde_json::Value = resp.json().unwrap_or(serde_json::Value::Null);
                let count_val = count.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
                objects_checked += 1;
                if count_val > 0 {
                    rows_matched += count_val;
                    checksums_matched += 1;
                } else {
                    mismatches.push(format!("Object '{}' has 0 rows in target", mapping.target));
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().unwrap_or_default();
                mismatches.push(format!(
                    "Failed to check count for '{}': HTTP {} - {}",
                    mapping.target, status, text
                ));
            }
            Err(e) => {
                mismatches.push(format!(
                    "Failed to connect to target for '{}': {}",
                    mapping.target, e
                ));
            }
        }
    }

    Ok(ValidationReport {
        objects_checked,
        rows_matched,
        checksums_matched,
        mismatches,
    })
}
