//! db_query / db_get / db_run con la semántica de better-sqlite3 12 (prepare(sql).all/get/run(params ?? [])).
use rusqlite::{Connection, Statement, types::Null};
use serde::Serialize;
use serde_json::{Map, Value};

pub const ERR_BIND_TYPE: &str =
    "SQLite3 can only bind numbers, strings, bigints, buffers, and null";
pub const ERR_NOT_READER: &str = "This statement does not return data. Use run() instead";
pub const ERR_NO_STATEMENTS: &str = "The supplied SQL string contains no statements";
pub const ERR_MULTIPLE: &str = "The supplied SQL string contains more than one statement";
pub const ERR_TOO_FEW: &str = "Too few parameter values were provided";
pub const ERR_TOO_MANY: &str = "Too many parameter values were provided";
pub const ERR_MISSING_NAMED: &str = "Missing named parameters";

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub changes: u64,
    pub last_insert_rowid: i64,
}

pub fn sqlite_message(e: rusqlite::Error) -> String {
    match e {
        rusqlite::Error::SqliteFailure(err, msg) => msg.unwrap_or_else(|| err.to_string()),
        rusqlite::Error::SqlInputError { msg, .. } => msg,
        rusqlite::Error::MultipleStatement => ERR_MULTIPLE.into(),
        other => other.to_string(),
    }
}

/// `db.prepare(sql)` + enlace posicional de `params ?? []` con las reglas del Binder de better-sqlite3:
/// los valores van, en orden, a los parámetros anónimos (`?`); `?NNN`, `:x`, `@x`, `$x` cuentan como nombrados.
fn prepare<'c>(conn: &'c Connection, sql: &str, params: &[Value]) -> Result<Statement<'c>, String> {
    let mut stmt = conn.prepare(sql).map_err(sqlite_message)?;
    if stmt.expanded_sql().is_none() {
        return Err(ERR_NO_STATEMENTS.into());
    } // vacío, espacios, ';' o solo comentarios
    let total = stmt.parameter_count();
    let anonymous: Vec<usize> = (1..=total)
        .filter(|&i| stmt.parameter_name(i).is_none())
        .collect();
    for (k, p) in params.iter().enumerate() {
        let Some(&idx) = anonymous.get(k) else {
            return Err(ERR_TOO_MANY.into());
        };
        let r = match p {
            Value::Null => stmt.raw_bind_parameter(idx, Null),
            Value::Number(n) => stmt.raw_bind_parameter(idx, n.as_f64().unwrap_or(f64::NAN)), // siempre REAL
            Value::String(s) => stmt.raw_bind_parameter(idx, s.as_str()),
            _ => return Err(ERR_BIND_TYPE.into()), // booleanos, arrays y objetos
        };
        r.map_err(sqlite_message)?;
    }
    if params.len() < total {
        return Err(if anonymous.len() < total {
            ERR_MISSING_NAMED.into()
        } else {
            ERR_TOO_FEW.into()
        });
    }
    Ok(stmt)
}

fn row_to_object(row: &rusqlite::Row<'_>, names: &[String]) -> Map<String, Value> {
    let mut obj = Map::new();
    for (i, name) in names.iter().enumerate() {
        obj.insert(
            name.clone(),
            crate::sqlvalue::value_ref_to_json(row.get_ref(i).unwrap()),
        ); // duplicado: gana el último
    }
    obj
}

pub fn db_query(
    conn: &Connection,
    sql: &str,
    params: Option<Vec<Value>>,
) -> Result<Vec<Map<String, Value>>, String> {
    let mut stmt = prepare(conn, sql, &params.unwrap_or_default())?;
    if stmt.column_count() == 0 {
        return Err(ERR_NOT_READER.into());
    }
    let names: Vec<String> = stmt.column_names().map(String::from).collect();
    let mut rows = stmt.raw_query();
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(sqlite_message)? {
        out.push(row_to_object(row, &names));
    }
    Ok(out)
}

pub fn db_get(
    conn: &Connection,
    sql: &str,
    params: Option<Vec<Value>>,
) -> Result<Option<Map<String, Value>>, String> {
    let mut stmt = prepare(conn, sql, &params.unwrap_or_default())?;
    if stmt.column_count() == 0 {
        return Err(ERR_NOT_READER.into());
    }
    let names: Vec<String> = stmt.column_names().map(String::from).collect();
    let mut rows = stmt.raw_query();
    Ok(rows
        .next()
        .map_err(sqlite_message)?
        .map(|row| row_to_object(row, &names)))
}

pub fn db_run(
    conn: &Connection,
    sql: &str,
    params: Option<Vec<Value>>,
) -> Result<RunResult, String> {
    let mut stmt = prepare(conn, sql, &params.unwrap_or_default())?;
    let before = conn.total_changes();
    {
        let mut rows = stmt.raw_query();
        rows.next().map_err(sqlite_message)?; // un solo paso, como better-sqlite3
    } // drop → sqlite3_reset
    let changes = if conn.total_changes() == before {
        0
    } else {
        conn.changes()
    };
    Ok(RunResult {
        changes,
        last_insert_rowid: conn.last_insert_rowid(),
    })
}
