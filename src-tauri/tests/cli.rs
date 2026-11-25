//! La CLI de punta a punta: proceso real, BD temporal y salida JSON.
use serde_json::{Value, json};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_cuentacobro-cli");

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout debe ser JSON")
}

#[test]
fn opera_la_bd_con_las_reglas_de_la_app() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("database.sqlite");
    let cli = |args: &[&str]| {
        Command::new(BIN)
            .arg("--db")
            .arg(&db)
            .args(args)
            .output()
            .unwrap()
    };

    let client = cli(&[
        "run",
        "INSERT INTO clients (name) VALUES (?)",
        r#"["ACME"]"#,
    ]);
    assert!(client.status.success(), "{client:?}");
    assert_eq!(
        stdout_json(&client),
        json!({ "changes": 1, "lastInsertRowid": 1 })
    );
    cli(&[
        "run",
        "INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, 1)",
        r#"["Banco", "Ahorros", "123"]"#,
    ]);

    let model = stdout_json(&cli(&["cuenta-open"]));
    assert_eq!(model["value"]["cuenta"]["number"], "1");
    assert_eq!(model["value"]["cuenta"]["bankAccountId"], 1);

    let create = json!({ "kind": "create", "cuenta": {
        "number": "1", "date": "2026-11-11", "clientId": 1, "bankAccountId": 1, "notes": "",
        "concepts": [{ "description": "Servicio", "quantity": "2", "price": "50000.25" }],
    }})
    .to_string();
    let saved = cli(&["cuenta-save", &create]);
    assert!(saved.status.success(), "{saved:?}");
    assert_eq!(stdout_json(&saved)["value"]["total"], "100000.50");

    // El rechazo del dominio sale por stdout (para leer el detalle) y con código 1.
    let repeated = cli(&["cuenta-save", &create]);
    assert_eq!(repeated.status.code(), Some(1));
    assert_eq!(stdout_json(&repeated)["error"]["code"], "NUMBER_IN_USE");

    let row = cli(&[
        "get",
        "SELECT total, status FROM invoices WHERE number = ?",
        "[1]",
    ]);
    assert_eq!(
        stdout_json(&row),
        json!({ "total": 100000.5, "status": "draft" })
    );

    let backup = dir.path().join("respaldo.sqlite");
    let backup = backup.to_str().unwrap();
    assert!(cli(&["backup", backup]).status.success());
    assert_eq!(
        cli(&["backup", backup]).status.code(),
        Some(1),
        "no sobrescribe"
    );

    // Mismos errores que el renderer: booleanos no se enlazan.
    let bad = cli(&["run", "SELECT ?", "[true]"]);
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stderr).contains(cuentacobro_lib::generic::ERR_BIND_TYPE));
    assert_eq!(cli(&["no-existe"]).status.code(), Some(1));
}

/// Sin --db no crea la BD: la app se saltaría la importación de la 1.x en su primer arranque.
#[cfg(unix)]
#[test]
fn sin_db_no_crea_la_bd_de_la_app() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(BIN)
        .args(["query", "SELECT 1"])
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("datos"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("abre CuentaCobro una vez"));
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn abonos_comparten_el_saldo_del_editor_y_revierten_fallos_de_estado() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("abonos.sqlite");
    let cli = |args: &[&str]| {
        Command::new(BIN)
            .arg("--db")
            .arg(&path)
            .args(args)
            .output()
            .unwrap()
    };
    let ok = |args: &[&str]| {
        let output = cli(args);
        assert!(output.status.success(), "{output:?}");
        stdout_json(&output)
    };
    ok(&["cuenta-list"]);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(
        "INSERT INTO clients(name) VALUES ('Cliente');
         INSERT INTO bank_accounts(bank, account_type, account_number) VALUES ('Banco','Ahorros','1');
         INSERT INTO invoices(number,date,client_id,total,bank_account_id) VALUES (1,'2026-11-30',1,10.30,1);
         INSERT INTO invoice_items(invoice_id,description,quantity,price) VALUES (1,'Trabajo',1,10.30);"
    ).unwrap();
    let payment =
        |amount: &str| json!({"invoiceId":1, "date":"2026-11-30", "amount":amount}).to_string();
    let count = || {
        conn.query_row("SELECT COUNT(*) FROM invoice_payments", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap()
    };
    let status = || {
        conn.query_row("SELECT status FROM invoices", [], |r| r.get::<_, String>(0))
            .unwrap()
    };
    let fail_update = || {
        conn.execute_batch(
            "CREATE TRIGGER fallo_estado BEFORE UPDATE OF status ON invoices
         BEGIN SELECT RAISE(ABORT,'fallo de prueba'); END;",
        )
        .unwrap()
    };

    for amount in ["0", "-1", "0.001", "1e2", "90071992547409.91"] {
        assert!(!cli(&["abono-add", &payment(amount)]).status.success());
    }
    assert!(
        !cli(&["abono-add", &payment("1.00").replace("11-30", "11-31")])
            .status
            .success()
    );
    fail_update();
    assert!(!cli(&["abono-add", &payment("10.20")]).status.success());
    assert_eq!(count(), 0);
    assert_eq!(status(), "draft");
    conn.execute_batch("DROP TRIGGER fallo_estado").unwrap();

    let partial = ok(&["abono-add", &payment("10.20")]);
    assert_eq!(partial["balance"], "0.10");
    assert_eq!(status(), "partially_paid");
    let paid = ok(&["abono-add", &payment("0.10")]);
    assert_eq!(
        paid,
        json!({"total":"10.30","paidAmount":"10.30","balance":"0.00","paymentStatus":"paid"})
    );
    assert_eq!(ok(&["cuenta-list"])[0]["summary"], paid);
    assert_eq!(ok(&["cuenta-open", "1"])["value"]["summary"], paid);
    assert_eq!(status(), "paid");
    assert_eq!(count(), 2);

    let overpaid = ok(&["abono-add", &payment("0.25")]);
    assert_eq!(overpaid["paidAmount"], "10.55");
    assert_eq!(overpaid["balance"], "0.00");
    assert_eq!(status(), "paid");
    assert!(!cli(&["abono-delete", "2", "3"]).status.success());
    assert_eq!(ok(&["abono-delete", "1", "3"]), paid);
    fail_update();
    assert!(!cli(&["abono-delete", "1", "2"]).status.success());
    assert_eq!(status(), "paid");
    assert_eq!(count(), 2);
    conn.execute_batch("DROP TRIGGER fallo_estado").unwrap();
    assert_eq!(ok(&["abono-delete", "1", "2"]), partial);
    assert_eq!(status(), "partially_paid");
    let empty = ok(&["abono-delete", "1", "1"]);
    assert_eq!(empty["balance"], "10.30");
    assert_eq!(status(), "draft");
}

#[test]
fn repara_una_base_anterior_una_vez_sin_tocar_los_abonos() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("antigua.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(cuentacobro_lib::db::SCHEMA_SQL).unwrap();
    conn.execute_batch(
        "INSERT INTO clients(name) VALUES ('Cliente');
         INSERT INTO invoices(number,date,client_id,total,status)
             VALUES (1,'2026-11-30',1,10.30,'partially_paid'),(2,'2026-11-30',1,8,'paid'),
                    (3,'2026-11-30',1,1,'partially_paid');
         INSERT INTO invoice_payments(invoice_id,date,amount)
             VALUES (1,'2026-11-30',10.20),(1,'2026-11-30',0.10),(3,'2026-11-30',4);
         CREATE TABLE cambios_estado(id INTEGER);
         CREATE TRIGGER cuenta_cambios AFTER UPDATE OF status ON invoices
             BEGIN INSERT INTO cambios_estado VALUES(NEW.id); END;",
    )
    .unwrap();
    let cli = |args: &[&str]| {
        Command::new(BIN)
            .arg("--db")
            .arg(&path)
            .args(args)
            .output()
            .unwrap()
    };
    for _ in 0..2 {
        let rows = cli(&["query", "SELECT status FROM invoices ORDER BY id"]);
        assert!(rows.status.success(), "{rows:?}");
        assert_eq!(
            stdout_json(&rows),
            json!([{"status":"paid"},{"status":"draft"},{"status":"paid"}])
        );
    }
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM cambios_estado", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM invoice_payments", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
