//! PrimusDB writer implementations.
//!
//! Writers connect to a running PrimusDB server via its REST API
//! and handle schema creation and data insertion.

use super::target::{PrimusMigrationWriter, RestWriter};
use crate::Result;

/// Create a [`PrimusMigrationWriter`] for the given engine type.
///
/// Supported engines are dispatched to [`RestWriter`]:
/// - `relational`
/// - `columnar`
/// - `document`
/// - `keyvalue`
/// - `vector`
///
/// Unknown engine types return [`crate::Error::Unsupported`].
pub fn create_writer(
    engine: &str,
    server_url: &str,
    namespace: &str,
) -> Result<Box<dyn PrimusMigrationWriter>> {
    match engine {
        "relational" | "columnar" | "document" | "keyvalue" | "vector" => {
            Ok(Box::new(RestWriter::new(
                server_url.to_string(),
                namespace.to_string(),
            )))
        }
        other => Err(crate::Error::Unsupported(format!(
            "Unsupported target engine '{}'. Supported engines: relational, columnar, document, keyvalue, vector",
            other
        ))),
    }
}
