//! Casos límite adicionales E1–E8, comparados con la salida capturada de la versión JS (1.0.11).
use cuentacobro_lib::{cuenta_de_cobro, db};
use rusqlite::params;
use serde_json::{Map, Value, json};
fn j<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}
/// Salida exacta de electron/cuentaDeCobro.ts para la misma secuencia (Electron 41, better-sqlite3 12.10).
const EXPECTED: &str = r#"{
 "E1_create_sin_default": {"ok":true,"value":{"target":{"kind":"create"},"cuenta":{"number":"1","date":"__TODAY__","clientId":null,"bankAccountId":null,"notes":"","concepts":[{"description":"","quantity":"1","price":""}]},"clientes":[{"id":2,"name":"A cliente"},{"id":1,"name":"b cliente"}],"cuentasBancarias":[{"id":2,"bank":"Alfa","accountType":"Corriente","accountNumber":"2","isDefault":false},{"id":1,"bank":"zeta","accountType":"Ahorros","accountNumber":"1","isDefault":false}],"summary":{"total":"0.00","paidAmount":"0.00","balance":"0.00","paymentStatus":"draft"},"diagnostics":[]}},
 "E2_sin_conceptos": {"ok":true,"value":{"target":{"kind":"edit","id":1},"cuenta":{"number":"10","date":"2026-08-27","clientId":1,"bankAccountId":1,"notes":"","concepts":[]},"clientes":[{"id":2,"name":"A cliente"},{"id":1,"name":"b cliente"}],"cuentasBancarias":[{"id":2,"bank":"Alfa","accountType":"Corriente","accountNumber":"2","isDefault":false},{"id":1,"bank":"zeta","accountType":"Ahorros","accountNumber":"1","isDefault":false}],"summary":{"total":"5.00","paidAmount":"0.00","balance":"5.00","paymentStatus":"draft"},"diagnostics":[{"code":"INVALID_PERSISTED_CONCEPTS"}]}},
 "E3_abonos_parciales": {"ok":true,"value":{"target":{"kind":"edit","id":2},"cuenta":{"number":"11","date":"2026-08-27","clientId":1,"bankAccountId":1,"notes":"","concepts":[{"description":"x","quantity":"1","price":"1"}]},"clientes":[{"id":2,"name":"A cliente"},{"id":1,"name":"b cliente"}],"cuentasBancarias":[{"id":2,"bank":"Alfa","accountType":"Corriente","accountNumber":"2","isDefault":false},{"id":1,"bank":"zeta","accountType":"Ahorros","accountNumber":"1","isDefault":false}],"summary":{"total":"1.00","paidAmount":"0.50","balance":"0.50","paymentStatus":"partially_paid"},"diagnostics":[{"code":"INVALID_PERSISTED_PAYMENTS"}]}},
 "E4_sobrepago_y_total_3_decimales": {"ok":true,"value":{"target":{"kind":"edit","id":3},"cuenta":{"number":"12","date":"2026-08-27","clientId":1,"bankAccountId":1,"notes":"","concepts":[{"description":"x","quantity":"1","price":"0.675"}]},"clientes":[{"id":2,"name":"A cliente"},{"id":1,"name":"b cliente"}],"cuentasBancarias":[{"id":2,"bank":"Alfa","accountType":"Corriente","accountNumber":"2","isDefault":false},{"id":1,"bank":"zeta","accountType":"Ahorros","accountNumber":"1","isDefault":false}],"summary":{"total":"0.68","paidAmount":"2.00","balance":"0.00","paymentStatus":"paid"},"diagnostics":[]}},
 "E5_save_edit_sobrepago": {"ok":true,"value":{"id":3,"number":12,"total":"2.00","status":"paid"}},
 "E6_suggested_null": {"ok":false,"error":{"code":"NUMBER_IN_USE","number":11,"suggestedNumber":null}},
 "E7_create_next_vacio": {"ok":true,"value":{"target":{"kind":"create"},"cuenta":{"number":"","date":"__TODAY__","clientId":null,"bankAccountId":2,"notes":"","concepts":[{"description":"","quantity":"1","price":""}]},"clientes":[{"id":2,"name":"A cliente"},{"id":1,"name":"b cliente"}],"cuentasBancarias":[{"id":2,"bank":"Alfa","accountType":"Corriente","accountNumber":"2","isDefault":true},{"id":1,"bank":"zeta","accountType":"Ahorros","accountNumber":"1","isDefault":false}],"summary":{"total":"0.00","paidAmount":"0.00","balance":"0.00","paymentStatus":"draft"},"diagnostics":[]}},
 "E8_numero_texto": {"ok":true,"value":{"target":{"kind":"edit","id":1},"cuenta":{"number":"abc","date":"2026-08-27","clientId":1,"bankAccountId":1,"notes":"","concepts":[]},"clientes":[{"id":2,"name":"A cliente"},{"id":1,"name":"b cliente"}],"cuentasBancarias":[{"id":2,"bank":"Alfa","accountType":"Corriente","accountNumber":"2","isDefault":true},{"id":1,"bank":"zeta","accountType":"Ahorros","accountNumber":"1","isDefault":false}],"summary":{"total":"5.00","paidAmount":"0.00","balance":"5.00","paymentStatus":"draft"},"diagnostics":[{"code":"INVALID_PERSISTED_ACCOUNT"},{"code":"INVALID_PERSISTED_CONCEPTS"}]}}
}"#;

#[test]
fn edge_cases_match_js() {
    let expected: Map<String, Value> =
        serde_json::from_str(&EXPECTED.replace("__TODAY__", &cuenta_de_cobro::today_local_date()))
            .unwrap();
    let dir = std::env::temp_dir().join(format!("edge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut conn = db::open_connection(&dir.join("database.sqlite")).unwrap();
    db::init_db(&conn).unwrap();
    conn.execute("INSERT INTO clients (name) VALUES (?)", ["b cliente"])
        .unwrap();
    conn.execute("INSERT INTO clients (name) VALUES (?)", ["A cliente"])
        .unwrap();
    conn.execute("INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, ?)", params!["zeta", "Ahorros", "1", 0.0]).unwrap();
    conn.execute("INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, ?)", params!["Alfa", "Corriente", "2", 0.0]).unwrap();
    let ins = |c: &rusqlite::Connection, n: f64, total: Option<f64>| {
        c.execute("INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, '2026-08-27', 1, 1, ?, NULL, 'draft')", params![n, total]).unwrap();
        c.last_insert_rowid()
    };
    let open = |c: &rusqlite::Connection, t: Value| {
        j(&cuenta_de_cobro::open(
            c,
            serde_json::from_value(t).unwrap(),
        ))
    };
    let mut got = Map::new();
    got.insert(
        "E1_create_sin_default".into(),
        open(&conn, json!({"kind":"create"})),
    );
    let a = ins(&conn, 10.0, Some(5.0));
    got.insert(
        "E2_sin_conceptos".into(),
        open(&conn, json!({"kind":"edit","id":a})),
    );
    let b = ins(&conn, 11.0, Some(1.0));
    conn.execute("INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, 'x', 1, 1)", [b]).unwrap();
    for amount in [0.5, -1.0, 0.25] {
        conn.execute(
            "INSERT INTO invoice_payments (invoice_id, date, amount) VALUES (?, '2026-08-27', ?)",
            params![b, amount],
        )
        .unwrap();
    }
    got.insert(
        "E3_abonos_parciales".into(),
        open(&conn, json!({"kind":"edit","id":b})),
    );
    let c = ins(&conn, 12.0, Some(0.675));
    conn.execute("INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, 'x', 1, 0.675)", [c]).unwrap();
    conn.execute(
        "INSERT INTO invoice_payments (invoice_id, date, amount) VALUES (?, '2026-08-27', 2)",
        [c],
    )
    .unwrap();
    got.insert(
        "E4_sobrepago_y_total_3_decimales".into(),
        open(&conn, json!({"kind":"edit","id":c})),
    );
    conn.execute("UPDATE bank_accounts SET is_default = 1 WHERE id = 2", [])
        .unwrap();
    got.insert("E5_save_edit_sobrepago".into(), j(&cuenta_de_cobro::save(Some(&mut conn), serde_json::from_value(json!({"kind":"edit","id":c,"cuenta":{"number":"12","date":"2026-08-27","clientId":1,"bankAccountId":2,"notes":"","concepts":[{"description":"x","quantity":"1","price":"2"}]}})).unwrap())));
    conn.execute(
        "UPDATE invoices SET number = 9007199254740991 WHERE id = ?",
        [a],
    )
    .unwrap();
    got.insert("E6_suggested_null".into(), j(&cuenta_de_cobro::save(Some(&mut conn), serde_json::from_value(json!({"kind":"create","cuenta":{"number":"11","date":"2026-08-27","clientId":1,"bankAccountId":1,"notes":"","concepts":[{"description":"x","quantity":"1","price":"1"}]}})).unwrap())));
    got.insert(
        "E7_create_next_vacio".into(),
        open(&conn, json!({"kind":"create"})),
    );
    conn.execute("UPDATE invoices SET number = 'abc' WHERE id = ?", [a])
        .unwrap();
    got.insert(
        "E8_numero_texto".into(),
        open(&conn, json!({"kind":"edit","id":a})),
    );
    for (k, v) in &expected {
        assert_eq!(&got[k], v, "{k}");
    }
    assert_eq!(got.len(), expected.len());
    std::fs::remove_dir_all(&dir).ok();
}

/// X01–X04: orden de validación y contratos de entrada (salidas capturadas de la versión JS).
#[test]
fn validacion_y_contrato_de_entrada() {
    let dir = std::env::temp_dir().join(format!("edge-x-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut conn = db::open_connection(&dir.join("database.sqlite")).unwrap();
    db::init_db(&conn).unwrap();
    let mut save = |cmd: Value| {
        j(&cuenta_de_cobro::save(
            Some(&mut conn),
            serde_json::from_value(cmd).unwrap(),
        ))
    };
    // X01: todo inválido → orden fijo de issues
    assert_eq!(
        save(
            json!({"kind":"create","cuenta":{"number":"x","date":"2026-02-30","clientId":null,"bankAccountId":0,"notes":"","concepts":[]}})
        ),
        json!({"ok":false,"error":{"code":"VALIDATION_FAILED","issues":[{"field":"number","code":"INVALID_POSITIVE_INTEGER"},{"field":"date","code":"INVALID_DATE"},{"field":"clientId","code":"REQUIRED"},{"field":"bankAccountId","code":"REQUIRED"},{"field":"concepts","code":"AT_LEAST_ONE_CONCEPT_REQUIRED"}]}})
    );
    // X02: issues por concepto (pueden repetirse)
    assert_eq!(
        save(
            json!({"kind":"create","cuenta":{"number":"1","date":"2026-01-01","clientId":1.5,"bankAccountId":-1,"notes":"","concepts":[
        {"description":"  ","quantity":"abc","price":"0"},
        {"description":"ok","quantity":format!("1{}.1", "0".repeat(30)),"price":"1"},
        {"description":"ok","quantity":"1","price":"0.004"},
        {"description":"ok","quantity":"100000000000","price":"100000000"},
        {"description":"ok","quantity":"1","price":"90071992547409"},
        {"description":"ok","quantity":"1","price":"0.91"},
        {"description":"ok","quantity":"1","price":"1"}]}})
        ),
        json!({"ok":false,"error":{"code":"VALIDATION_FAILED","issues":[{"field":"clientId","code":"REQUIRED"},{"field":"bankAccountId","code":"REQUIRED"},{"field":"concepts.0.description","code":"REQUIRED"},{"field":"concepts.0.quantity","code":"INVALID_POSITIVE_DECIMAL"},{"field":"concepts.0.price","code":"INVALID_POSITIVE_DECIMAL"},{"field":"concepts.1.quantity","code":"AMOUNT_OUT_OF_RANGE"},{"field":"concepts.2.price","code":"SUBTOTAL_ROUNDS_TO_ZERO"},{"field":"concepts.3.price","code":"AMOUNT_OUT_OF_RANGE"},{"field":"concepts","code":"AMOUNT_OUT_OF_RANGE"},{"field":"concepts","code":"AMOUNT_OUT_OF_RANGE"}]}})
    );
    // X03: total no representable en REAL
    assert_eq!(
        save(
            json!({"kind":"create","cuenta":{"number":"1","date":"2026-01-01","clientId":1,"bankAccountId":1,"notes":"","concepts":[
        {"description":"a","quantity":"1","price":"90071992547409"},{"description":"b","quantity":"1","price":"0.91"}]}})
        ),
        json!({"ok":false,"error":{"code":"VALIDATION_FAILED","issues":[{"field":"concepts","code":"AMOUNT_OUT_OF_RANGE"}]}})
    );
    assert_eq!(
        save(
            json!({"kind":"create","cuenta":{"number":"1","date":"2026-01-01","clientId":1,"bankAccountId":1,"notes":"","concepts":[]}})
        ),
        json!({"ok":false,"error":{"code":"VALIDATION_FAILED","issues":[{"field":"concepts","code":"AT_LEAST_ONE_CONCEPT_REQUIRED"}]}})
    );
    // X04: ids de edición no válidos y BD vacía
    let open = |t: Value| {
        j(&cuenta_de_cobro::open(
            &conn,
            serde_json::from_value(t).unwrap(),
        ))
    };
    assert_eq!(
        open(json!({"kind":"edit","id":null})),
        json!({"ok":false,"error":{"code":"CUENTA_DE_COBRO_NOT_FOUND","id":null}})
    ); // NaN viaja como null
    assert_eq!(
        open(json!({"kind":"edit"})),
        json!({"ok":false,"error":{"code":"CUENTA_DE_COBRO_NOT_FOUND","id":null}})
    );
    assert_eq!(
        open(json!({"kind":"edit","id":1.5})),
        json!({"ok":false,"error":{"code":"CUENTA_DE_COBRO_NOT_FOUND","id":1.5}})
    );
    assert_eq!(
        open(json!({"kind":"create"})),
        json!({"ok":true,"value":{"target":{"kind":"create"},"cuenta":{"number":"1","date":cuenta_de_cobro::today_local_date(),"clientId":null,"bankAccountId":null,"notes":"","concepts":[{"description":"","quantity":"1","price":""}]},"clientes":[],"cuentasBancarias":[],"summary":{"total":"0.00","paidAmount":"0.00","balance":"0.00","paymentStatus":"draft"},"diagnostics":[]}})
    );
    // Entrada mal tipada: JS rechazaba la promesa ("cuenta.number.trim is not a function"); aquí falla la deserialización → invoke rechaza.
    assert!(serde_json::from_value::<cuentacobro_lib::types::SaveCuentaDeCobroCommand>(json!({"kind":"create","cuenta":{"number":5,"date":"2026-01-01","clientId":1,"bankAccountId":1,"notes":"","concepts":[]}})).is_err());
    drop(conn);
    std::fs::remove_dir_all(&dir).ok();
}
