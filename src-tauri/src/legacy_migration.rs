//! Migración única de la base de datos heredada de la versión Electron (≤ 1.x).
//!
//! Electron guardaba `database.sqlite` en `app.getPath('userData')`, que es
//! `<config_dir del SO>/<app.name>`. El `package.json` empaquetado por
//! electron-builder NO lleva `productName`, así que `app.name` era el `name`
//! del package.json: `cuenta-cobro` (verificado en el `app.asar` de 1.0.10 y en
//! un equipo real). `CuentaCobro` queda como candidato defensivo.
//!
//! Reglas (no negociables):
//! - Solo se migra si la BD nueva NO existe.
//! - Nunca se modifica ni se borra nada en la carpeta heredada.
//! - Se escribe en un archivo temporal, se valida y se renombra atómicamente.
//! - Si existe una BD heredada válida pero no se pudo copiar, se devuelve error
//!   para que el arranque NO cree una BD vacía (que impediría reintentar).

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};

/// Nombre del archivo de BD (igual en Electron y en Tauri).
pub const DB_FILE_NAME: &str = "database.sqlite";

/// Carpetas `userData` de Electron, en orden de preferencia, relativas a
/// `app.path().config_dir()` (= `app.getPath('appData')` de Electron).
pub const LEGACY_DIR_NAMES: [&str; 2] = ["cuenta-cobro", "CuentaCobro"];

/// Marcador escrito junto a la BD nueva tras una migración exitosa.
pub const MARKER_FILE_NAME: &str = "legacy-migration.json";

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";
/// Tamaño mínimo de una BD SQLite no vacía (página mínima = 512 bytes).
const MIN_SQLITE_SIZE: u64 = 512;
const TEMP_SUFFIX: &str = ".migrating";
/// Sufijos de archivos auxiliares que se copian en el modo de copia cruda.
/// `-shm` NO se copia: es un índice reconstruible y contiene estado de locks.
const SIDECAR_SUFFIXES: [&str; 2] = ["-journal", "-wal"];
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMethod {
    /// `VACUUM INTO` desde una conexión de solo lectura (instantánea consistente).
    VacuumInto,
    /// Copia de archivos (principal + `-journal`/`-wal`) y recuperación sobre la copia.
    RawFileCopy,
}

impl CopyMethod {
    fn as_str(self) -> &'static str {
        match self {
            CopyMethod::VacuumInto => "vacuum_into",
            CopyMethod::RawFileCopy => "raw_file_copy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// La BD nueva ya existía: no se tocó nada.
    AlreadyPresent,
    /// No hay BD heredada utilizable: se arrancará con BD vacía.
    NoLegacyFound,
    /// Se importó la BD heredada.
    Migrated { source: PathBuf, method: CopyMethod },
}

#[derive(Debug)]
pub enum MigrationError {
    /// No se pudo preparar la carpeta destino.
    TargetDir {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Había al menos una BD heredada con cabecera SQLite, pero ninguna se pudo
    /// copiar y validar. El llamador NO debe crear una BD vacía.
    LegacyPresentButFailed { attempts: Vec<(PathBuf, String)> },
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationError::TargetDir { path, source } => {
                write!(
                    f,
                    "no se pudo crear la carpeta de datos {}: {source}",
                    path.display()
                )
            }
            MigrationError::LegacyPresentButFailed { attempts } => {
                write!(f, "no se pudo importar la base de datos anterior")?;
                for (path, reason) in attempts {
                    write!(f, "; {}: {reason}", path.display())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for MigrationError {}

/// Rutas candidatas de la BD heredada, en orden.
pub fn legacy_candidates(config_dir: &Path) -> Vec<PathBuf> {
    LEGACY_DIR_NAMES
        .iter()
        .map(|dir| config_dir.join(dir).join(DB_FILE_NAME))
        .collect()
}

/// Importa la primera BD heredada válida a `target_db` si `target_db` no existe.
pub fn migrate_legacy_database(
    target_db: &Path,
    candidates: &[PathBuf],
) -> Result<MigrationOutcome, MigrationError> {
    if fs::symlink_metadata(target_db).is_ok() {
        log::debug!("BD encontrada en {}; no se migra", target_db.display());
        return Ok(MigrationOutcome::AlreadyPresent);
    }

    if let Some(parent) = target_db.parent() {
        fs::create_dir_all(parent).map_err(|source| MigrationError::TargetDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let temp_db = with_suffix(target_db, TEMP_SUFFIX);
    let mut attempts: Vec<(PathBuf, String)> = Vec::new();

    for candidate in candidates {
        match looks_like_sqlite(candidate) {
            Ok(true) => {}
            Ok(false) => {
                if candidate.exists() {
                    log::warn!(
                        "Se ignora {}: no tiene cabecera SQLite válida",
                        candidate.display()
                    );
                }
                continue;
            }
            Err(err) => {
                log::warn!("No se pudo leer {}: {err}", candidate.display());
                attempts.push((candidate.clone(), format!("lectura: {err}")));
                continue;
            }
        }

        log::info!("BD heredada encontrada en {}", candidate.display());
        remove_temp_files(&temp_db);

        let method = match copy_with_vacuum_into(candidate, &temp_db) {
            Ok(()) => CopyMethod::VacuumInto,
            Err(vacuum_err) => {
                log::warn!(
                    "VACUUM INTO falló para {} ({vacuum_err}); se intenta copia de archivos",
                    candidate.display()
                );
                remove_temp_files(&temp_db);
                if let Err(copy_err) = copy_raw_files(candidate, &temp_db) {
                    log::warn!("Copia de archivos falló: {copy_err}");
                    attempts.push((
                        candidate.clone(),
                        format!("vacuum: {vacuum_err}; copia: {copy_err}"),
                    ));
                    remove_temp_files(&temp_db);
                    continue;
                }
                CopyMethod::RawFileCopy
            }
        };

        if let Err(validation_err) = validate_and_finalize(&temp_db) {
            log::warn!(
                "La copia de {} no pasó la validación: {validation_err}",
                candidate.display()
            );
            attempts.push((candidate.clone(), format!("validación: {validation_err}")));
            remove_temp_files(&temp_db);
            continue;
        }

        if let Err(rename_err) = fs::rename(&temp_db, target_db) {
            log::warn!("No se pudo renombrar la copia: {rename_err}");
            attempts.push((candidate.clone(), format!("renombrado: {rename_err}")));
            remove_temp_files(&temp_db);
            continue;
        }

        write_marker(target_db, candidate, method);
        log::info!(
            "BD heredada importada desde {} ({}) a {}",
            candidate.display(),
            method.as_str(),
            target_db.display()
        );
        return Ok(MigrationOutcome::Migrated {
            source: candidate.clone(),
            method,
        });
    }

    if attempts.is_empty() {
        log::info!("No se encontró BD heredada de Electron; se inicia con BD nueva");
        Ok(MigrationOutcome::NoLegacyFound)
    } else {
        Err(MigrationError::LegacyPresentButFailed { attempts })
    }
}

/// `true` si `path` es un archivo regular con cabecera "SQLite format 3\0".
fn looks_like_sqlite(path: &Path) -> std::io::Result<bool> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };
    if !metadata.is_file() || metadata.len() < MIN_SQLITE_SIZE {
        return Ok(false);
    }
    let mut header = [0u8; 16];
    File::open(path)?.read_exact(&mut header)?;
    Ok(&header == SQLITE_HEADER)
}

/// Instantánea consistente sin escribir en la carpeta heredada (la BD de
/// Electron usa journal en modo DELETE: abrirla en solo lectura no crea archivos).
fn copy_with_vacuum_into(source: &Path, temp_db: &Path) -> rusqlite::Result<()> {
    let conn = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    let temp = temp_db.to_string_lossy().into_owned();
    conn.execute("VACUUM INTO ?1", [temp])?;
    Ok(())
}

/// Plan B (p. ej. journal "caliente" tras un cierre abrupto de Electron, que
/// impide leer en solo lectura): copiar los archivos y recuperar sobre la copia.
fn copy_raw_files(source: &Path, temp_db: &Path) -> std::io::Result<()> {
    fs::copy(source, temp_db)?;
    for suffix in SIDECAR_SUFFIXES {
        let sidecar = with_suffix(source, suffix);
        if sidecar.is_file() {
            fs::copy(&sidecar, with_suffix(temp_db, suffix))?;
        }
    }
    Ok(())
}

/// Abre la copia en lectura/escritura (SQLite revierte un journal caliente o
/// aplica el WAL sobre la COPIA), comprueba integridad y esquema, y la deja
/// como un único archivo en modo DELETE listo para renombrar.
fn validate_and_finalize(temp_db: &Path) -> Result<(), String> {
    let conn = Connection::open_with_flags(
        temp_db,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    conn.busy_timeout(BUSY_TIMEOUT).map_err(|e| e.to_string())?;

    let quick_check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if quick_check != "ok" {
        return Err(format!("quick_check: {quick_check}"));
    }

    let has_profile: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'profile'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if has_profile != 1 {
        return Err("no contiene la tabla 'profile' de CuentaCobro".into());
    }

    let mode: String = conn
        .pragma_update_and_check(None, "journal_mode", "DELETE", |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if !mode.eq_ignore_ascii_case("delete") {
        return Err(format!(
            "no se pudo fijar journal_mode=DELETE (quedó {mode})"
        ));
    }
    conn.close().map_err(|(_, e)| e.to_string())?;

    for suffix in SIDECAR_SUFFIXES {
        if with_suffix(temp_db, suffix).exists() {
            return Err(format!("quedó un archivo {suffix} sin consolidar"));
        }
    }
    Ok(())
}

fn write_marker(target_db: &Path, source: &Path, method: CopyMethod) {
    let Some(dir) = target_db.parent() else {
        return;
    };
    let migrated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let marker = serde_json::json!({
        "source": source.to_string_lossy(),
        "method": method.as_str(),
        "migratedAtUnix": migrated_at,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "noticeAcknowledged": false,
    });
    let path = dir.join(MARKER_FILE_NAME);
    match serde_json::to_vec_pretty(&marker) {
        Ok(bytes) => {
            if let Err(err) = fs::write(&path, bytes) {
                log::warn!("No se pudo escribir {}: {err}", path.display());
            }
        }
        Err(err) => log::warn!("No se pudo serializar el marcador: {err}"),
    }
}

fn remove_temp_files(temp_db: &Path) {
    let _ = fs::remove_file(temp_db);
    for suffix in ["-journal", "-wal", "-shm"] {
        let _ = fs::remove_file(with_suffix(temp_db, suffix));
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut os = path.as_os_str().to_owned();
    os.push(suffix);
    PathBuf::from(os)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_legacy_db(path: &Path, clients: &[&str]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE profile (id INTEGER PRIMARY KEY CHECK (id = 1), name TEXT);
             INSERT INTO profile (id, name) VALUES (1, 'Mi Nombre');
             CREATE TABLE clients (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);",
        )
        .unwrap();
        for name in clients {
            conn.execute("INSERT INTO clients (name) VALUES (?1)", [name])
                .unwrap();
        }
    }

    fn client_names(path: &Path) -> Vec<String> {
        let conn = Connection::open(path).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM clients ORDER BY id")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn dir_snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut entries: Vec<(String, Vec<u8>)> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap())
            .map(|e| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    fs::read(e.path()).unwrap(),
                )
            })
            .collect();
        entries.sort();
        entries
    }

    struct Fixture {
        _tmp: TempDir,
        config_dir: PathBuf,
        target: PathBuf,
    }

    fn fixture() -> Fixture {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        let target = tmp
            .path()
            .join("data")
            .join("com.cuentacobro.app")
            .join(DB_FILE_NAME);
        fs::create_dir_all(&config_dir).unwrap();
        Fixture {
            _tmp: tmp,
            config_dir,
            target,
        }
    }

    #[test]
    fn candidates_follow_electron_user_data_names() {
        let base = Path::new("/base");
        assert_eq!(
            legacy_candidates(base),
            vec![
                PathBuf::from("/base/cuenta-cobro/database.sqlite"),
                PathBuf::from("/base/CuentaCobro/database.sqlite"),
            ]
        );
    }

    #[test]
    fn migrates_with_vacuum_into_and_leaves_original_untouched() {
        let f = fixture();
        let legacy = f.config_dir.join("cuenta-cobro").join(DB_FILE_NAME);
        create_legacy_db(&legacy, &["Ana", "Luis"]);
        let before = dir_snapshot(legacy.parent().unwrap());

        let outcome =
            migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap();

        assert_eq!(
            outcome,
            MigrationOutcome::Migrated {
                source: legacy.clone(),
                method: CopyMethod::VacuumInto
            }
        );
        assert_eq!(client_names(&f.target), vec!["Ana", "Luis"]);
        assert_eq!(dir_snapshot(legacy.parent().unwrap()), before);
        assert!(f.target.with_file_name(MARKER_FILE_NAME).is_file());
        assert!(!with_suffix(&f.target, TEMP_SUFFIX).exists());
    }

    #[test]
    fn prefers_cuenta_cobro_over_product_name_folder() {
        let f = fixture();
        create_legacy_db(
            &f.config_dir.join("cuenta-cobro").join(DB_FILE_NAME),
            &["Real"],
        );
        create_legacy_db(
            &f.config_dir.join("CuentaCobro").join(DB_FILE_NAME),
            &["Otro"],
        );

        migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap();

        assert_eq!(client_names(&f.target), vec!["Real"]);
    }

    #[test]
    fn falls_back_to_second_candidate_when_first_is_not_sqlite() {
        let f = fixture();
        let bogus = f.config_dir.join("cuenta-cobro").join(DB_FILE_NAME);
        fs::create_dir_all(bogus.parent().unwrap()).unwrap();
        fs::write(&bogus, vec![b'x'; 4096]).unwrap();
        create_legacy_db(
            &f.config_dir.join("CuentaCobro").join(DB_FILE_NAME),
            &["Dos"],
        );

        migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap();

        assert_eq!(client_names(&f.target), vec!["Dos"]);
    }

    #[test]
    fn does_nothing_when_target_exists() {
        let f = fixture();
        create_legacy_db(
            &f.config_dir.join("cuenta-cobro").join(DB_FILE_NAME),
            &["Viejo"],
        );
        create_legacy_db(&f.target, &["Nuevo"]);

        let outcome =
            migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap();

        assert_eq!(outcome, MigrationOutcome::AlreadyPresent);
        assert_eq!(client_names(&f.target), vec!["Nuevo"]);
    }

    #[test]
    fn reports_no_legacy_when_nothing_exists() {
        let f = fixture();
        let outcome =
            migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap();
        assert_eq!(outcome, MigrationOutcome::NoLegacyFound);
        assert!(!f.target.exists());
    }

    #[test]
    fn rejects_sqlite_without_cuenta_cobro_schema() {
        let f = fixture();
        let legacy = f.config_dir.join("cuenta-cobro").join(DB_FILE_NAME);
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        Connection::open(&legacy)
            .unwrap()
            .execute_batch("CREATE TABLE otra (x INTEGER); INSERT INTO otra VALUES (1);")
            .unwrap();

        let err =
            migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap_err();

        assert!(matches!(err, MigrationError::LegacyPresentButFailed { .. }));
        assert!(!f.target.exists());
        assert!(!with_suffix(&f.target, TEMP_SUFFIX).exists());
    }

    /// Simula un cierre abrupto de Electron a mitad de transacción: queda un
    /// `-journal` caliente. La lectura en solo lectura falla, se usa la copia de
    /// archivos y SQLite revierte la transacción sobre la copia.
    #[test]
    fn recovers_hot_journal_on_the_copy_only() {
        let f = fixture();
        let live = f.config_dir.join("live").join(DB_FILE_NAME);
        create_legacy_db(&live, &["Confirmado"]);

        let conn = Connection::open(&live).unwrap();
        conn.execute_batch("PRAGMA journal_mode = DELETE; PRAGMA cache_size = 1;")
            .unwrap();
        conn.execute_batch("BEGIN;").unwrap();
        for i in 0..400 {
            conn.execute(
                "INSERT INTO clients (name) VALUES (?1)",
                [format!("sin-confirmar-{i}-{}", "x".repeat(900))],
            )
            .unwrap();
        }
        let journal = with_suffix(&live, "-journal");
        assert!(
            journal.is_file(),
            "la transacción debe haber creado el journal"
        );

        let legacy_dir = f.config_dir.join("cuenta-cobro");
        fs::create_dir_all(&legacy_dir).unwrap();
        let legacy = legacy_dir.join(DB_FILE_NAME);
        fs::copy(&live, &legacy).unwrap();
        fs::copy(&journal, with_suffix(&legacy, "-journal")).unwrap();
        conn.execute_batch("ROLLBACK;").unwrap();
        drop(conn);
        let before = dir_snapshot(&legacy_dir);

        let outcome =
            migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap();

        assert_eq!(
            outcome,
            MigrationOutcome::Migrated {
                source: legacy.clone(),
                method: CopyMethod::RawFileCopy
            }
        );
        assert_eq!(client_names(&f.target), vec!["Confirmado"]);
        assert_eq!(dir_snapshot(&legacy_dir), before);
        assert!(!with_suffix(&f.target, "-journal").exists());
    }

    #[test]
    fn migrates_wal_database_including_uncheckpointed_frames() {
        let f = fixture();
        let live = f.config_dir.join("live-wal").join(DB_FILE_NAME);
        create_legacy_db(&live, &["Base"]);
        let conn = Connection::open(&live).unwrap();
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))
            .unwrap();
        conn.execute_batch("PRAGMA wal_autocheckpoint = 0;")
            .unwrap();
        conn.execute("INSERT INTO clients (name) VALUES ('EnWal')", [])
            .unwrap();

        let legacy_dir = f.config_dir.join("cuenta-cobro");
        fs::create_dir_all(&legacy_dir).unwrap();
        let legacy = legacy_dir.join(DB_FILE_NAME);
        fs::copy(&live, &legacy).unwrap();
        fs::copy(with_suffix(&live, "-wal"), with_suffix(&legacy, "-wal")).unwrap();
        drop(conn);

        migrate_legacy_database(&f.target, &legacy_candidates(&f.config_dir)).unwrap();

        assert_eq!(client_names(&f.target), vec!["Base", "EnWal"]);
        assert!(!with_suffix(&f.target, "-wal").exists());
    }
}
