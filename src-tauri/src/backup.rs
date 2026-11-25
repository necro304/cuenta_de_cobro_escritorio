//! Instantáneas consistentes y restauración transaccional de la base de CuentaCobro.
use crate::{Database, db, generic::sqlite_message};
use rusqlite::{
    Connection, ErrorCode, MAIN_DB, OpenFlags, OptionalExtension,
    backup::{Backup, NO_PROGRESS, StepResult},
};
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

const SIDECARS: [&str; 3] = ["-journal", "-wal", "-shm"];

fn io_message(error: io::Error) -> String {
    error.to_string()
}

fn same_file(a: &Path, b: &Path) -> io::Result<bool> {
    #[cfg(unix)]
    {
        // Abrir y cerrar un descriptor adicional puede soltar los bloqueos POSIX de SQLite.
        use std::os::unix::fs::MetadataExt;
        let a = fs::metadata(a)?;
        let b = fs::metadata(b)?;
        Ok((a.dev(), a.ino()) == (b.dev(), b.ino()))
    }
    #[cfg(windows)]
    {
        same_file::is_same_file(a, b)
    }
}

fn reject_live_file(live: &Path, selected: &Path) -> Result<(), String> {
    match same_file(live, selected) {
        Ok(true) => Err("Elige un archivo distinto de la base de datos activa.".into()),
        Ok(false) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

fn require_no_sidecars(path: &Path) -> Result<(), String> {
    for suffix in SIDECARS {
        let auxiliary = sidecar(path, suffix);
        match fs::symlink_metadata(&auxiliary) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
            Ok(_) => {
                return Err(format!(
                    "El destino conserva el archivo auxiliar {}. Cierra las conexiones a esa base antes de sustituirla.",
                    auxiliary.display()
                ));
            }
        }
    }
    Ok(())
}

fn integrity(conn: &Connection) -> Result<(), String> {
    let result: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(sqlite_message)?;
    if result != "ok" {
        return Err(format!(
            "El respaldo no superó la comprobación de integridad: {result}"
        ));
    }
    Ok(())
}

fn snapshot(source: &Connection, target: &Path) -> Result<NamedTempFile, String> {
    let dir = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = NamedTempFile::new_in(dir).map_err(io_message)?;
    let mut copy = Connection::open(temporary.path()).map_err(sqlite_message)?;
    // Un solo paso mantiene una única lectura aunque otra conexión confirme cambios en WAL.
    let step = Backup::new(source, &mut copy)
        .and_then(|backup| backup.step(-1))
        .map_err(sqlite_message)?;
    if step != StepResult::Done {
        return Err("La base de datos está ocupada. Vuelve a intentar la copia.".into());
    }
    copy.pragma_update(None, "trusted_schema", false)
        .map_err(sqlite_message)?;
    let mode: String = copy
        .pragma_update_and_check(None, "journal_mode", "DELETE", |r| r.get(0))
        .map_err(sqlite_message)?;
    if mode != "delete" {
        return Err("No se pudo consolidar el respaldo en un solo archivo.".into());
    }
    integrity(&copy)?;
    copy.close().map_err(|(_, e)| sqlite_message(e))?;
    Ok(temporary)
}

fn prepare_backup(conn: &Connection, target: &Path) -> Result<NamedTempFile, String> {
    let name = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if SIDECARS.iter().any(|suffix| name.ends_with(suffix)) {
        return Err("Elige un archivo distinto de un diario auxiliar de SQLite.".into());
    }
    let copy = snapshot(conn, target)?;
    copy.as_file().sync_all().map_err(io_message)?;
    require_no_sidecars(target)?;
    Ok(copy)
}

/// El diálogo de la aplicación confirma la sustitución del destino.
pub fn save(db: &Database, target: &Path) -> Result<(), String> {
    reject_live_file(&db.path, target)?;
    let guard = db.lock();
    let conn = guard.as_ref().ok_or(crate::commands::ERR_DB_CLOSED)?;
    prepare_backup(conn, target)?
        .persist(target)
        .map_err(|e| e.error.to_string())?;
    Ok(())
}

/// La CLI publica exclusivamente archivos nuevos, incluso si otro proceso crea el destino.
pub fn save_new(conn: &Connection, target: &Path) -> Result<(), String> {
    prepare_backup(conn, target)?
        .persist_noclobber(target)
        .map_err(|e| e.error.to_string())?;
    Ok(())
}

const APP_TABLES: [&str; 6] = [
    "profile",
    "clients",
    "invoices",
    "invoice_items",
    "invoice_payments",
    "bank_accounts",
];

/// Compara columnas, claves primarias y foráneas con la definición que usa esta versión.
/// El orden de las columnas difiere en bases actualizadas con ALTER TABLE.
fn table_shape(conn: &Connection, table: &str) -> rusqlite::Result<BTreeSet<String>> {
    conn.prepare(
        r#"SELECT json_array(name, lower(type), pk, "notnull") FROM pragma_table_info(?1)
           UNION ALL
           SELECT json_array("from", "table", "to", on_update, on_delete) FROM pragma_foreign_key_list(?1)"#,
    )?
    .query_map([table], |row| row.get(0))?
    .collect()
}

fn prepare_restore(path: &Path) -> Result<(), String> {
    let copy = Connection::open(path).map_err(sqlite_message)?;
    copy.pragma_update(None, "trusted_schema", false)
        .map_err(sqlite_message)?;
    // init_db también crea tablas. Verificar primero las que ya existían en Electron 1.x.
    for table in APP_TABLES {
        let is_table: bool = copy.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_list WHERE schema = 'main' AND type = 'table' AND name = ?1)",
            [table], |r| r.get(0),
        ).map_err(sqlite_message)?;
        if !is_table {
            return Err(format!(
                "El respaldo no contiene la tabla {table} de CuentaCobro."
            ));
        }
    }
    db::init_db(&copy).map_err(sqlite_message)?;
    let expected = Connection::open_in_memory().map_err(sqlite_message)?;
    db::init_db(&expected).map_err(sqlite_message)?;
    for table in APP_TABLES {
        let required = table_shape(&expected, table).map_err(sqlite_message)?;
        if !required.is_subset(&table_shape(&copy, table).map_err(sqlite_message)?) {
            return Err(format!(
                "El esquema de la tabla {table} es incompatible con CuentaCobro."
            ));
        }
    }
    let orphan: Option<String> = copy
        .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
        .optional()
        .map_err(sqlite_message)?;
    if let Some(table) = orphan {
        return Err(format!(
            "El respaldo contiene una referencia inexistente en {table}."
        ));
    }
    copy.close().map_err(|(_, e)| sqlite_message(e))?;
    Ok(())
}

fn restore_into(live: &mut Connection, path: &Path) -> rusqlite::Result<()> {
    let mode: String = live.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
    if mode == "wal" {
        // La API de respaldo de SQLite no permite cambiar el tamaño de página de un WAL.
        let page_size: i64 = live.pragma_query_value(None, "page_size", |r| r.get(0))?;
        let copy = Connection::open(path)?;
        let source_size: i64 = copy.pragma_query_value(None, "page_size", |r| r.get(0))?;
        if source_size != page_size {
            copy.pragma_update(None, "page_size", page_size)?;
            copy.execute_batch("VACUUM")?;
        }
    }
    live.restore(MAIN_DB, path, NO_PROGRESS)
}

pub fn restore(db: &Database, source: &Path) -> Result<(), String> {
    reject_live_file(&db.path, source)?;
    let input = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(sqlite_message)?;
    let copy = snapshot(&input, &db.path)?;
    drop(input);
    prepare_restore(copy.path())?;

    let mut guard = db.lock();
    if let Some(live) = guard.as_mut() {
        // SQLite publica la copia en una transacción sobre el mismo archivo. Una CLI
        // conectada conserva su bloqueo y pasa a ver la restauración una vez confirmada.
        return restore_into(live, copy.path()).map_err(sqlite_message);
    }

    // Abrir en escritura puede limpiar un WAL incluso con la cabecera dañada.
    // Una base cerrada con diarios pendientes debe poder leerse antes de recuperarla.
    if let Err(sidecar_error) = require_no_sidecars(&db.path) {
        let probe = Connection::open_with_flags(&db.path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .and_then(|conn| {
                conn.query_row("SELECT count(*) FROM sqlite_schema", [], |r| {
                    r.get::<_, i64>(0)
                })
            });
        if probe.is_err() {
            return Err(sidecar_error);
        }
    }
    match db::open_connection(&db.path).and_then(|mut live| {
        restore_into(&mut live, copy.path())?;
        Ok(live)
    }) {
        Ok(live) => {
            *guard = Some(live);
            Ok(())
        }
        Err(e)
            if matches!(
                e.sqlite_error_code(),
                Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase)
            ) =>
        {
            // Al arrancar con cabecera dañada SQLite no admite siquiera una transacción.
            // La CLI tampoco puede operar sobre ella. Solo aquí se reemplaza el archivo.
            require_no_sidecars(&db.path)?;
            copy.as_file().sync_all().map_err(io_message)?;
            copy.persist(&db.path).map_err(|e| e.error.to_string())?;
            Ok(()) // db_restore reinicia la aplicación y vuelve a abrirla.
        }
        Err(e) => Err(sqlite_message(e)),
    }
}
