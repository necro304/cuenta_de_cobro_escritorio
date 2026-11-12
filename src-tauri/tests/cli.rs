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
