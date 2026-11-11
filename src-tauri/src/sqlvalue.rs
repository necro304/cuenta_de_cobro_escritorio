//! Conversión de valores SQLite a JSON/JS con la semántica de better-sqlite3 (safeIntegers = false).
use rusqlite::types::ValueRef;
use serde_json::Value;

pub fn value_ref_to_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::from(i),
        ValueRef::Real(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::Array(b.iter().map(|x| Value::from(*x)).collect()),
    }
}

/// Valor leído de una columna, con el tipo de almacenamiento real de SQLite.
#[derive(Debug, Clone, PartialEq)]
pub enum Sql {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Sql {
    pub fn from_ref(v: ValueRef<'_>) -> Sql {
        match v {
            ValueRef::Null => Sql::Null,
            ValueRef::Integer(i) => Sql::Int(i),
            ValueRef::Real(f) => Sql::Real(f),
            ValueRef::Text(t) => Sql::Text(String::from_utf8_lossy(t).into_owned()),
            ValueRef::Blob(b) => Sql::Blob(b.to_vec()),
        }
    }
    /// typeof value === 'number' en JS → Some(número). INTEGER se convierte a f64 (con pérdida > 2^53, como better-sqlite3).
    pub fn as_js_number(&self) -> Option<f64> {
        match self {
            Sql::Int(i) => Some(*i as f64),
            Sql::Real(f) => Some(*f),
            _ => None,
        }
    }
    /// String(value) de JS.
    pub fn to_js_string(&self) -> String {
        match self {
            Sql::Null => "null".into(),
            Sql::Int(i) => crate::money::js_number_to_string(*i as f64),
            Sql::Real(f) => crate::money::js_number_to_string(*f),
            Sql::Text(s) => s.clone(),
            Sql::Blob(b) => String::from_utf8_lossy(b).into_owned(),
        }
    }
    /// Boolean(value) de JS.
    pub fn js_truthy(&self) -> bool {
        match self {
            Sql::Null => false,
            Sql::Int(i) => *i != 0,
            Sql::Real(f) => *f != 0.0 && !f.is_nan(),
            Sql::Text(s) => !s.is_empty(),
            Sql::Blob(_) => true,
        }
    }
    pub fn to_json(&self) -> Value {
        match self {
            Sql::Null => Value::Null,
            Sql::Int(i) => Value::from(*i),
            Sql::Real(f) => serde_json::Number::from_f64(*f)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            Sql::Text(s) => Value::String(s.clone()),
            Sql::Blob(b) => Value::Array(b.iter().map(|x| Value::from(*x)).collect()),
        }
    }
}

impl rusqlite::types::ToSql for Sql {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        use rusqlite::types::{ToSqlOutput, Value as V};
        Ok(match self {
            Sql::Null => ToSqlOutput::Owned(V::Null),
            // better-sqlite3 enlaza todo número JS como REAL (double)
            Sql::Int(i) => ToSqlOutput::Owned(V::Real(*i as f64)),
            Sql::Real(f) => ToSqlOutput::Owned(V::Real(*f)),
            Sql::Text(s) => ToSqlOutput::Borrowed(ValueRef::Text(s.as_bytes())),
            Sql::Blob(b) => ToSqlOutput::Borrowed(ValueRef::Blob(b)),
        })
    }
}

pub const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
/// Number.isSafeInteger
pub fn is_safe_integer(x: f64) -> bool {
    x.is_finite() && x.fract() == 0.0 && x.abs() <= MAX_SAFE_INTEGER
}
