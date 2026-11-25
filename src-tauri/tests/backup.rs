use cuentacobro_lib::{Database, backup, db};
use rusqlite::Connection;
use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

fn database(path: &Path, client: &str) -> Database {
    let conn = db::open_connection(path).unwrap();
    db::init_db(&conn).unwrap();
    conn.execute("INSERT INTO clients(name) VALUES(?)", [client])
        .unwrap();
    Database {
        path: path.into(),
        conn: Arc::new(Mutex::new(Some(conn))),
    }
}

fn client_name(conn: &Connection) -> String {
    conn.query_row("SELECT name FROM clients WHERE id=1", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn respaldo_incluye_abono_confirmado_en_wal_de_otra_conexion() {
    let dir = tempfile::tempdir().unwrap();
    let live = database(&dir.path().join("cuentas.sqlite"), "Cliente");
    let guard = live.lock();
    let app = guard.as_ref().unwrap();
    app.execute_batch(
        "INSERT INTO invoices(id, number, date, client_id, total) VALUES(1, 1, '2026-11-18', 1, 100);
         PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;",
    ).unwrap();
    let cli = Connection::open(&live.path).unwrap();
    cli.execute_batch(
        "PRAGMA wal_autocheckpoint=0;
         INSERT INTO invoice_payments(invoice_id,date,amount) VALUES(1, '2026-11-18', 25);",
    )
    .unwrap();

    let total = |conn: &Connection| {
        conn.query_row(
            "SELECT coalesce(sum(amount),0) FROM invoice_payments",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    };
    let raw = dir.path().join("solo-principal.sqlite");
    fs::copy(&live.path, &raw).unwrap();
    assert_eq!(total(&Connection::open(raw).unwrap()), 0);

    let target = dir.path().join("copia.sqlite");
    fs::write(&target, b"respaldo anterior").unwrap();
    drop(guard);
    backup::save(&live, &target).unwrap();
    assert_eq!(total(&Connection::open(target).unwrap()), 25);
    assert!(!dir.path().join("copia.sqlite-wal").exists());
}

#[test]
fn fallo_de_respaldo_preserva_el_destino_y_la_cli_no_reemplaza_un_archivo_vacio() {
    let dir = tempfile::tempdir().unwrap();
    let live = database(&dir.path().join("cuentas.sqlite"), "Vigente");
    let previous = dir.path().join("anterior.sqlite");
    fs::write(&previous, b"anterior").unwrap();
    let bad = dir.path().join("danada.sqlite");
    fs::write(&bad, b"esto no es SQLite").unwrap();
    let bad = Database {
        conn: Arc::new(Mutex::new(Some(Connection::open(&bad).unwrap()))),
        path: bad,
    };
    assert!(backup::save(&bad, &previous).is_err());
    assert_eq!(fs::read(&previous).unwrap(), b"anterior");

    let empty = dir.path().join("vacio.sqlite");
    fs::write(&empty, []).unwrap();
    let cli = std::process::Command::new(env!("CARGO_BIN_EXE_cuentacobro-cli"))
        .arg("--db")
        .arg(&live.path)
        .arg("backup")
        .arg(&empty)
        .output()
        .unwrap();
    assert!(!cli.status.success());
    assert!(fs::read(empty).unwrap().is_empty());
}

#[test]
fn restaurar_electron_migra_solo_la_copia_y_conserva_la_conexion_de_la_cli_en_wal() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("electron.sqlite");
    let original = Connection::open(&source).unwrap();
    original.execute_batch("PRAGMA page_size=1024").unwrap();
    db::init_db(&original).unwrap();
    original
        .execute_batch(
            "ALTER TABLE clients DROP COLUMN city;
         ALTER TABLE invoices DROP COLUMN bank_account_id;
         ALTER TABLE profile DROP COLUMN document_type;
         ALTER TABLE profile DROP COLUMN rut;
         ALTER TABLE profile DROP COLUMN signature;
         ALTER TABLE profile DROP COLUMN signature_color;
         ALTER TABLE profile DROP COLUMN signature_mode;
         ALTER TABLE profile DROP COLUMN default_template;
         INSERT INTO clients(name) VALUES('Heredado');",
        )
        .unwrap();
    drop(original);
    let original_bytes = fs::read(&source).unwrap();
    let live = database(&dir.path().join("cuentas.sqlite"), "Anterior");
    live.lock()
        .as_ref()
        .unwrap()
        .execute_batch("PRAGMA journal_mode=WAL")
        .unwrap();
    let cli = Connection::open(&live.path).unwrap();
    assert_eq!(client_name(&cli), "Anterior");

    backup::restore(&live, &source).unwrap();
    assert_eq!(fs::read(source).unwrap(), original_bytes);
    assert_eq!(client_name(&cli), "Heredado");
    cli.execute(
        "UPDATE clients SET city = 'Cali', name = 'Actualizado' WHERE id=1",
        [],
    )
    .unwrap();
    assert_eq!(client_name(live.lock().as_ref().unwrap()), "Actualizado");
}

#[test]
fn respaldo_corrupto_ajeno_o_incompatible_conserva_los_datos_y_la_conexion() {
    let dir = tempfile::tempdir().unwrap();
    let live = database(&dir.path().join("cuentas.sqlite"), "Vigente");
    let truncated = dir.path().join("truncada.sqlite");
    fs::copy(&live.path, &truncated).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&truncated)
        .unwrap()
        .set_len(113)
        .unwrap();
    let foreign = dir.path().join("otra.sqlite");
    Connection::open(&foreign)
        .unwrap()
        .execute_batch("CREATE TABLE profile(id INTEGER, name TEXT)")
        .unwrap();
    let incompatible = dir.path().join("incompleta.sqlite");
    let other = database(&incompatible, "Otro");
    other
        .lock()
        .as_ref()
        .unwrap()
        .execute_batch("ALTER TABLE invoice_items DROP COLUMN quantity")
        .unwrap();
    drop(other);
    for source in [&truncated, &foreign, &incompatible, &live.path] {
        assert!(
            backup::restore(&live, source).is_err(),
            "{}",
            source.display()
        );
        let guard = live.lock();
        let conn = guard.as_ref().unwrap();
        assert_eq!(client_name(conn), "Vigente");
        conn.execute("UPDATE clients SET phone='123' WHERE id=1", [])
            .unwrap();
    }
}

#[test]
fn transaccion_de_otra_conexion_bloquea_la_restauracion_sin_perder_escrituras() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("respaldo.sqlite");
    drop(database(&source, "Restaurado"));
    let live = database(&dir.path().join("cuentas.sqlite"), "Anterior");
    live.lock()
        .as_ref()
        .unwrap()
        .busy_timeout(Duration::ZERO)
        .unwrap();
    let cli = Connection::open(&live.path).unwrap();
    cli.execute_batch("BEGIN IMMEDIATE; UPDATE clients SET name='Desde CLI' WHERE id=1")
        .unwrap();
    assert!(backup::restore(&live, &source).is_err());
    cli.execute_batch("COMMIT").unwrap();
    assert_eq!(client_name(live.lock().as_ref().unwrap()), "Desde CLI");
    backup::restore(&live, &source).unwrap();
    assert_eq!(client_name(&cli), "Restaurado");
}

#[test]
fn restaurar_recupera_el_archivo_danado_al_arrancar_con_la_conexion_cerrada() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("respaldo.sqlite");
    drop(database(&source, "Recuperado"));
    let live = Database {
        path: dir.path().join("cuentas.sqlite"),
        conn: Arc::new(Mutex::new(None)),
    };
    fs::write(&live.path, b"cabecera ilegible").unwrap();
    backup::restore(&live, &source).unwrap();
    assert_eq!(
        client_name(&db::open_connection(&live.path).unwrap()),
        "Recuperado"
    );
}

#[test]
fn restaurar_conexion_cerrada_preserva_los_auxiliares_si_el_destino_esta_danado() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("respaldo.sqlite");
    drop(database(&source, "Recuperado"));
    let live = Database {
        path: dir.path().join("cuentas.sqlite"),
        conn: Arc::new(Mutex::new(None)),
    };
    let wal = dir.path().join("cuentas.sqlite-wal");
    fs::write(&live.path, b"cabecera ilegible").unwrap();
    fs::write(&wal, b"diario pendiente").unwrap();
    assert!(backup::restore(&live, &source).is_err());
    assert_eq!(fs::read(&live.path).unwrap(), b"cabecera ilegible");
    assert_eq!(fs::read(&wal).unwrap(), b"diario pendiente");
}
