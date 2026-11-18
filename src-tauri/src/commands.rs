use crate::{Database, cuenta_de_cobro, generic, types::*};
use serde_json::{Map, Value};
use tauri::State;

pub const ERR_DB_CLOSED: &str = "The database connection is not open";

#[tauri::command]
pub async fn clear_billing_data(db: State<'_, Database>) -> Result<(), String> {
    with_conn(&db, |conn| {
        crate::db::clear_billing_data(conn).map_err(generic::sqlite_message)
    })
    .await
}

async fn with_conn<T: Send + 'static>(
    db: &Database,
    f: impl FnOnce(&mut rusqlite::Connection) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let db = db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = db.lock();
        let conn = guard.as_mut().ok_or_else(|| ERR_DB_CLOSED.to_string())?;
        f(conn)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn db_query(
    db: State<'_, Database>,
    sql: String,
    params: Option<Vec<Value>>,
) -> Result<Vec<Map<String, Value>>, String> {
    with_conn(&db, move |c| generic::db_query(c, &sql, params)).await
}
#[tauri::command]
pub async fn db_get(
    db: State<'_, Database>,
    sql: String,
    params: Option<Vec<Value>>,
) -> Result<Option<Map<String, Value>>, String> {
    with_conn(&db, move |c| generic::db_get(c, &sql, params)).await
}
#[tauri::command]
pub async fn db_run(
    db: State<'_, Database>,
    sql: String,
    params: Option<Vec<Value>>,
) -> Result<generic::RunResult, String> {
    with_conn(&db, move |c| generic::db_run(c, &sql, params)).await
}
#[tauri::command]
pub async fn cuenta_de_cobro_open(
    db: State<'_, Database>,
    target: CuentaDeCobroTarget,
) -> Result<CuentaDeCobroResult<CuentaDeCobroEditorModel>, String> {
    Ok(
        with_conn(&db, move |c| Ok(cuenta_de_cobro::open(c, target)))
            .await
            .unwrap_or(CuentaDeCobroResult::Err(CuentaDeCobroError::StorageFailure)),
    )
}
#[tauri::command]
pub async fn cuenta_de_cobro_save(
    db: State<'_, Database>,
    command: SaveCuentaDeCobroCommand,
) -> Result<CuentaDeCobroResult<SaveCuentaDeCobroReceipt>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = db.lock();
        cuenta_de_cobro::save(guard.as_mut(), command)
    })
    .await
    .map_err(|e| e.to_string())
}
