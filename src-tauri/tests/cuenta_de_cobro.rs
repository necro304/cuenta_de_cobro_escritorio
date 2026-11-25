//! Port de electron/cuentaDeCobro.test.ts. Un único #[test] secuencial: los casos dependen del estado anterior.
use cuentacobro_lib::{cuenta_de_cobro, db};
use rusqlite::{Connection, params};
use serde_json::{Value, json};

fn to_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}
fn open(conn: &Connection, target: Value) -> Value {
    to_json(&cuenta_de_cobro::open(
        conn,
        serde_json::from_value(target).unwrap(),
    ))
}
fn save(conn: &mut Connection, command: Value) -> Value {
    to_json(&cuenta_de_cobro::save(
        Some(conn),
        serde_json::from_value(command).unwrap(),
    ))
}
/// Equivalente a `{ ...base, ...patch }` de JS.
fn spread(base: &Value, patch: Value) -> Value {
    let mut out = base.clone();
    for (k, v) in patch.as_object().unwrap() {
        out[k] = v.clone();
    }
    out
}
fn insert_invoice(
    conn: &Connection,
    number: f64,
    date: &str,
    client: Option<f64>,
    bank: Option<f64>,
    total: Option<f64>,
) -> i64 {
    conn.execute("INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![number, date, client, bank, total, "", "draft"]).unwrap();
    conn.last_insert_rowid()
}
fn insert_item(conn: &Connection, invoice_id: i64, description: &str, quantity: f64, price: f64) {
    conn.execute(
        "INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)",
        params![invoice_id as f64, description, quantity, price],
    )
    .unwrap();
}

#[test]
fn harness_cuenta_de_cobro() {
    let dir = std::env::temp_dir().join(format!("cuenta-de-cobro-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut conn = db::open_connection(&dir.join("database.sqlite")).unwrap();
    db::init_db(&conn).unwrap();
    let today = cuenta_de_cobro::today_local_date();

    // T01
    let color: String = conn
        .query_row(
            "SELECT signature_color FROM profile WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(color, "#245f46");

    conn.execute(
        "INSERT INTO clients (name) VALUES (?)",
        ["Cliente de prueba"],
    )
    .unwrap();
    let client = conn.last_insert_rowid();
    conn.execute("INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, 1)",
        ["Banco de prueba", "Ahorros", "123456"]).unwrap();
    let bank = conn.last_insert_rowid();
    let clientes = json!([{"id": 1, "name": "Cliente de prueba"}]);
    let cuentas = json!([{"id": 1, "bank": "Banco de prueba", "accountType": "Ahorros", "accountNumber": "123456", "isDefault": true}]);

    // T02-T04
    let opened = open(&conn, json!({"kind": "create"}));
    assert_eq!(
        opened,
        json!({"ok": true, "value": {
        "target": {"kind": "create"},
        "cuenta": {"number": "1", "date": today, "clientId": null, "bankAccountId": bank, "notes": "",
                   "concepts": [{"description": "", "quantity": "1", "price": ""}]},
        "clientes": clientes, "cuentasBancarias": cuentas,
        "summary": {"total": "0.00", "paidAmount": "0.00", "balance": "0.00", "paymentStatus": "draft"},
        "diagnostics": []}})
    );
    let base = opened["value"]["cuenta"].clone();

    // T05
    let saved = save(
        &mut conn,
        json!({"kind": "create", "cuenta": spread(&base, json!({"clientId": client, "concepts": [
        {"description": "Concepto A", "quantity": "1", "price": "0.335"},
        {"description": "Concepto B", "quantity": "1", "price": "0.335"}]}))}),
    );
    assert_eq!(
        saved,
        json!({"ok": true, "value": {"id": 1, "number": 1, "total": "0.68", "status": "draft"}})
    );

    // T06
    let reloaded = open(&conn, json!({"kind": "edit", "id": 1}));
    assert_eq!(
        reloaded,
        json!({"ok": true, "value": {
        "target": {"kind": "edit", "id": 1},
        "cuenta": {"number": "1", "date": today, "clientId": 1, "bankAccountId": 1, "notes": "",
                   "concepts": [{"description": "Concepto A", "quantity": "1", "price": "0.335"},
                                {"description": "Concepto B", "quantity": "1", "price": "0.335"}]},
        "clientes": clientes, "cuentasBancarias": cuentas,
        "summary": {"total": "0.68", "paidAmount": "0.00", "balance": "0.68", "paymentStatus": "draft"},
        "diagnostics": []}})
    );
    let reloaded_cuenta = reloaded["value"]["cuenta"].clone();

    // T07
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "create", "cuenta": spread(&base, json!({"number": "2", "clientId": client,
        "concepts": [{"description": "Sin valor", "quantity": "1", "price": "0"}]}))})
        ),
        json!({"ok": false, "error": {"code": "VALIDATION_FAILED", "issues": [{"field": "concepts.0.price", "code": "INVALID_POSITIVE_DECIMAL"}]}})
    );
    // T08
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "create", "cuenta": spread(&base, json!({"number": "1", "clientId": client,
        "concepts": [{"description": "Duplicado", "quantity": "1", "price": "1"}]}))})
        ),
        json!({"ok": false, "error": {"code": "NUMBER_IN_USE", "number": 1, "suggestedNumber": 2}})
    );
    // T09
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "create", "cuenta": spread(&base, json!({"number": "2", "clientId": 999,
        "concepts": [{"description": "Referencia", "quantity": "1", "price": "1"}]}))})
        ),
        json!({"ok": false, "error": {"code": "CLIENTE_NOT_FOUND", "id": 999}})
    );
    // T10
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "create", "cuenta": spread(&base, json!({"number": "2", "clientId": client, "bankAccountId": 999,
        "concepts": [{"description": "Referencia", "quantity": "1", "price": "1"}]}))})
        ),
        json!({"ok": false, "error": {"code": "CUENTA_BANCARIA_NOT_FOUND", "id": 999}})
    );

    // T11
    conn.execute(
        "INSERT INTO invoice_payments (invoice_id, date, amount, notes) VALUES (?, ?, ?, ?)",
        params![1.0, "2026-08-27", 0.5, ""],
    )
    .unwrap();
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "edit", "id": 1, "cuenta": spread(&reloaded_cuenta, json!({
        "concepts": [{"description": "Total reducido", "quantity": "1", "price": "0.40"}]}))})
        ),
        json!({"ok": false, "error": {"code": "TOTAL_BELOW_PAID_AMOUNT", "total": "0.40", "paidAmount": "0.50"}})
    );
    // T12
    let after_rejected = open(&conn, json!({"kind": "edit", "id": 1}));
    assert_eq!(
        after_rejected["value"]["summary"],
        json!({"total": "0.68", "paidAmount": "0.50", "balance": "0.18", "paymentStatus": "partially_paid"})
    );
    assert_eq!(
        after_rejected["value"]["cuenta"]["concepts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // T13
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "edit", "id": 1, "cuenta": spread(&reloaded_cuenta, json!({"notes": "  Cuenta pagada  ",
        "concepts": [{"description": "  Total pagado  ", "quantity": "1", "price": "0.50"}]}))})
        ),
        json!({"ok": true, "value": {"id": 1, "number": 1, "total": "0.50", "status": "paid"}})
    );
    // T14
    let paid = open(&conn, json!({"kind": "edit", "id": 1}));
    assert_eq!(paid["value"]["cuenta"]["notes"], json!("Cuenta pagada"));
    assert_eq!(
        paid["value"]["cuenta"]["concepts"],
        json!([{"description": "Total pagado", "quantity": "1", "price": "0.5"}])
    );
    assert_eq!(
        paid["value"]["summary"],
        json!({"total": "0.50", "paidAmount": "0.50", "balance": "0.00", "paymentStatus": "paid"})
    );
    let status: String = conn
        .query_row("SELECT status FROM invoices WHERE id = 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status, "paid");

    // T15
    let dup = insert_invoice(
        &conn,
        1.0,
        "2026-08-27",
        Some(client as f64),
        Some(bank as f64),
        Some(1.0),
    );
    insert_item(&conn, dup, "Histórico", 1.0, 1.0);
    assert_eq!(
        open(&conn, json!({"kind": "edit", "id": 1}))["value"]["diagnostics"],
        json!([{"code": "HISTORIC_NUMBER_CONFLICT", "number": 1}])
    );

    // T16
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    let invalid_historic = insert_invoice(
        &conn,
        4.0,
        "2026-08-27",
        Some(998.0),
        Some(999.0),
        Some(0.0),
    );
    insert_item(&conn, invalid_historic, "Sin valor", 1.0, 0.0);
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let ih = open(&conn, json!({"kind": "edit", "id": invalid_historic}));
    assert_eq!(
        ih["value"]["diagnostics"],
        json!([
        {"code": "MISSING_CLIENT_REFERENCE", "clientId": 998},
        {"code": "MISSING_BANK_ACCOUNT_REFERENCE", "bankAccountId": 999},
        {"code": "INVALID_PERSISTED_CONCEPTS"}])
    );
    assert_eq!(ih["value"]["cuenta"]["clientId"], Value::Null);
    assert_eq!(ih["value"]["cuenta"]["bankAccountId"], Value::Null);
    assert_eq!(
        ih["value"]["cuenta"]["concepts"],
        json!([{"description": "Sin valor", "quantity": "1", "price": "0"}])
    );
    assert_eq!(ih["value"]["summary"]["total"], json!("0.00"));

    // T17
    conn.execute_batch("CREATE TRIGGER fail_invoice_item_insert BEFORE INSERT ON invoice_items WHEN NEW.description = 'Forzar rollback' BEGIN SELECT RAISE(ABORT, 'forced test failure'); END;").unwrap();
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "create", "cuenta": spread(&base, json!({"number": "5", "clientId": client, "concepts": [
        {"description": "Primero", "quantity": "1", "price": "1"},
        {"description": "Forzar rollback", "quantity": "1", "price": "1"}]}))})
        ),
        json!({"ok": false, "error": {"code": "STORAGE_FAILURE"}})
    );
    let counts: (i64, i64) = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM invoices), (SELECT COUNT(*) FROM invoice_items)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(counts, (3, 3));
    conn.execute_batch("DROP TRIGGER fail_invoice_item_insert")
        .unwrap();
    // T18
    let retry = save(
        &mut conn,
        json!({"kind": "create", "cuenta": spread(&base, json!({"number": "5", "clientId": client,
        "concepts": [{"description": "Después del rollback", "quantity": "1", "price": "1"}]}))}),
    );
    assert_eq!(
        retry,
        json!({"ok": true, "value": {"id": 4, "number": 5, "total": "1.00", "status": "draft"}})
    );
    let retry_id = 4;

    // T19-T21
    let before = open(&conn, json!({"kind": "edit", "id": retry_id}));
    assert_eq!(before["ok"], json!(true));
    conn.execute_batch("CREATE TRIGGER fail_invoice_item_edit BEFORE INSERT ON invoice_items WHEN NEW.description = 'Forzar rollback de edición' BEGIN SELECT RAISE(ABORT, 'forced edit failure'); END;").unwrap();
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "edit", "id": retry_id, "cuenta": spread(&before["value"]["cuenta"], json!({
        "number": "8", "date": "2030-01-01", "notes": "No debe persistir",
        "concepts": [{"description": "Forzar rollback de edición", "quantity": "1", "price": "0.50"}]}))})
        ),
        json!({"ok": false, "error": {"code": "STORAGE_FAILURE"}})
    );
    conn.execute_batch("DROP TRIGGER fail_invoice_item_edit")
        .unwrap();
    let after = open(&conn, json!({"kind": "edit", "id": retry_id}));
    assert_eq!(after, before);
    assert_eq!(
        after["value"]["cuenta"]["concepts"][0]["description"],
        json!("Después del rollback")
    );

    // T22-T24
    let sci = save(
        &mut conn,
        json!({"kind": "create", "cuenta": spread(&base, json!({"number": "6", "clientId": client,
        "concepts": [{"description": "Decimal pequeño", "quantity": "100000", "price": "0.0000001"}]}))}),
    );
    assert_eq!(
        sci,
        json!({"ok": true, "value": {"id": 5, "number": 6, "total": "0.01", "status": "draft"}})
    );
    let sci_open = open(&conn, json!({"kind": "edit", "id": 5}));
    assert_eq!(
        sci_open["value"]["cuenta"]["concepts"],
        json!([{"description": "Decimal pequeño", "quantity": "100000", "price": "0.0000001"}])
    );
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "edit", "id": 5, "cuenta": sci_open["value"]["cuenta"]})
        ),
        json!({"ok": true, "value": {"id": 5, "number": 6, "total": "0.01", "status": "draft"}})
    );
    // T25
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "create", "cuenta": spread(&base, json!({"number": "7", "clientId": client,
        "concepts": [{"description": "No representable", "quantity": "1", "price": "90071992547409.91"}]}))})
        ),
        json!({"ok": false, "error": {"code": "VALIDATION_FAILED", "issues": [{"field": "concepts.0.price", "code": "AMOUNT_OUT_OF_RANGE"}]}})
    );

    // T26
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    let malformed = insert_invoice(&conn, 0.0, "2026-02-30", Some(client as f64), None, None);
    insert_item(&conn, malformed, "Reparable", 1.0, 1.0);
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let mo = open(&conn, json!({"kind": "edit", "id": malformed}));
    assert_eq!(
        mo["value"]["diagnostics"],
        json!([{"code": "INVALID_PERSISTED_ACCOUNT"}, {"code": "MISSING_BANK_ACCOUNT_REFERENCE", "bankAccountId": null}])
    );
    assert_eq!(mo["value"]["cuenta"]["number"], json!("0"));
    assert_eq!(mo["value"]["cuenta"]["date"], json!("2026-02-30"));
    assert_eq!(mo["value"]["summary"]["total"], json!("1.00"));

    // T27
    let inconsistent = insert_invoice(
        &conn,
        8.0,
        "2026-08-27",
        Some(client as f64),
        Some(bank as f64),
        Some(999.0),
    );
    insert_item(&conn, inconsistent, "Total real", 1.0, 1.0);
    let io = open(&conn, json!({"kind": "edit", "id": inconsistent}));
    assert_eq!(
        io["value"]["diagnostics"],
        json!([{"code": "INVALID_PERSISTED_ACCOUNT"}])
    );
    assert_eq!(io["value"]["summary"]["total"], json!("1.00"));

    // T28
    conn.execute(
        "INSERT INTO invoice_payments (invoice_id, date, amount, notes) VALUES (?, ?, ?, ?)",
        params![retry_id as f64, "2026-08-27", -1.0, ""],
    )
    .unwrap();
    let ip = open(&conn, json!({"kind": "edit", "id": retry_id}));
    assert_eq!(
        ip["value"]["diagnostics"],
        json!([{"code": "INVALID_PERSISTED_PAYMENTS"}])
    );
    assert_eq!(
        ip["value"]["summary"],
        json!({"total": "1.00", "paidAmount": "0.00", "balance": "1.00", "paymentStatus": "draft"})
    );

    // X05: editar una cuenta con abonos inválidos → STORAGE_FAILURE
    assert_eq!(
        save(
            &mut conn,
            json!({"kind": "edit", "id": retry_id, "cuenta": ip["value"]["cuenta"]})
        ),
        json!({"ok": false, "error": {"code": "STORAGE_FAILURE"}})
    );

    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}
