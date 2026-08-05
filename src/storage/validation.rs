//! Shared validation helpers for storage engines.
//!
//! Engines historically validated inline (and inconsistently). These helpers
//! centralize the numeric/vector and JSON-shape checks so every engine rejects
//! malformed input with the same error semantics instead of silently dropping
//! or coercing data.

use serde_json::Value;

use crate::Result;

/// Validates a numeric element is a finite f32.
fn finite_f32(n: f64) -> Result<f32> {
    let f = n as f32;
    if !f.is_finite() {
        return Err(crate::Error::ValidationError(
            "vector elements must be finite (no NaN or infinity)".into(),
        ));
    }
    Ok(f)
}

/// Parses a strict finite float vector from a JSON array.
///
/// Every element must be a number (integer or float); `null`, booleans,
/// strings, objects and nested arrays are rejected — they are never silently
/// dropped. NaN and ±infinity are rejected because they poison similarity
/// math and indexing.
pub fn parse_finite_vector(value: &Value) -> Result<Vec<f32>> {
    let arr = value
        .as_array()
        .ok_or_else(|| crate::Error::ValidationError("expected an array of numbers".into()))?;
    if arr.is_empty() {
        return Err(crate::Error::ValidationError(
            "vector must not be empty".into(),
        ));
    }
    let mut out = Vec::with_capacity(arr.len());
    for element in arr {
        match element {
            Value::Number(n) => {
                let f = n
                    .as_f64()
                    .ok_or_else(|| crate::Error::ValidationError("invalid numeric value".into()))?;
                out.push(finite_f32(f)?);
            }
            _ => {
                return Err(crate::Error::ValidationError(
                    "vector elements must be numbers".into(),
                ));
            }
        }
    }
    Ok(out)
}

/// Returns the dimension of a finite vector value.
pub fn vector_dimension(value: &Value) -> Result<usize> {
    parse_finite_vector(value).map(|v| v.len())
}

/// Requires `data` to be a JSON object (a row/record), never an array or
/// scalar.
pub fn ensure_object(data: &Value) -> Result<&serde_json::Map<String, Value>> {
    data.as_object()
        .ok_or_else(|| crate::Error::ValidationError("record must be a JSON object".into()))
}

/// True when the value contains no NaN/infinity numbers anywhere.
pub fn is_finite_json(value: &Value) -> bool {
    match value {
        Value::Number(n) => n.as_f64().is_none_or(f64::is_finite),
        Value::Array(items) => items.iter().all(is_finite_json),
        Value::Object(map) => map.values().all(is_finite_json),
        _ => true,
    }
}

/// Whether a JSON value is acceptable for a declared column type.
///
/// `Null` is always accepted (nullable columns). Numeric column types accept
/// JSON numbers, string/text/date column types accept JSON strings, booleans
/// accept booleans, and array/vector columns accept JSON arrays. `Json` and
/// unknown column types accept anything.
pub fn field_type_accepts(ft: &crate::storage::FieldType, value: &Value) -> bool {
    use crate::storage::FieldType as Ft;
    match value {
        Value::Null => true,
        Value::Bool(_) => matches!(ft, Ft::Boolean),
        Value::Number(_) => matches!(
            ft,
            Ft::Integer
                | Ft::Float
                | Ft::SmallInt
                | Ft::BigInt
                | Ft::Decimal(_, _)
                | Ft::Serial
                | Ft::BigSerial
                | Ft::Money
        ),
        Value::String(_) => matches!(
            ft,
            Ft::String
                | Ft::Text
                | Ft::Varchar(_)
                | Ft::Char(_)
                | Ft::Uuid
                | Ft::Date
                | Ft::DateTime
                | Ft::Timestamp
                | Ft::Time
                | Ft::Binary
                | Ft::Enum(_)
        ),
        Value::Array(_) => matches!(ft, Ft::Array(_) | Ft::Vector(_)),
        Value::Object(_) => matches!(ft, Ft::Json),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_finite_vector_ok() {
        let v = serde_json::json!([1.0, 2, 3.5]);
        let parsed = parse_finite_vector(&v).unwrap();
        assert_eq!(parsed, vec![1.0, 2.0, 3.5]);
    }

    #[test]
    fn test_parse_finite_vector_rejects_nan() {
        assert!(finite_f32(f64::NAN).is_err());
        assert!(finite_f32(f64::INFINITY).is_err());
        assert!(finite_f32(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn test_parse_finite_vector_rejects_non_numbers() {
        let v = serde_json::json!([1.0, "x", 2.0]);
        let err = parse_finite_vector(&v).unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));
    }

    #[test]
    fn test_parse_finite_vector_rejects_empty() {
        assert!(parse_finite_vector(&serde_json::json!([])).is_err());
    }
}
