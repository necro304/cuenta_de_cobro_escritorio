use cuentacobro_lib::system::{
    client_report_default_name, invoice_pdf_default_name, pdf_target_path,
};
#[test]
fn nombres_por_defecto() {
    assert_eq!(invoice_pdf_default_name(7.0), "cuenta_de_cobro_00007.pdf");
    assert_eq!(
        invoice_pdf_default_name(123456.0),
        "cuenta_de_cobro_123456.pdf"
    );
    assert_eq!(invoice_pdf_default_name(1.5), "cuenta_de_cobro_001.5.pdf");
    assert_eq!(
        client_report_default_name(3.0, "José Ñúñez & Cía. S.A.S."),
        "informe_jose_nunez_cia_s_a_s.pdf"
    );
    assert_eq!(
        client_report_default_name(3.0, "名前"),
        "informe_cliente_3.pdf"
    );
}

/// En Linux GTK confirma la sobrescritura de la ruta escrita (`factura`): la extensión `.pdf`
/// solo se añade si así no se pisa un archivo que el usuario no confirmó.
#[test]
fn ruta_del_pdf_sin_extension_no_pisa_otro_archivo() {
    let dir = tempfile::tempdir().unwrap();
    let elegido = dir.path().join("factura");
    let con_pdf = dir.path().join("factura.pdf");

    assert_eq!(pdf_target_path(elegido.clone()), con_pdf);

    std::fs::write(&con_pdf, b"otro documento").unwrap();
    assert_eq!(pdf_target_path(elegido.clone()), elegido);

    let con_extension = dir.path().join("cuenta.pdf");
    assert_eq!(pdf_target_path(con_extension.clone()), con_extension);
}

/// El renderer (`src/api/backend.ts`, CANCELLED_MESSAGE) compara este texto literal.
#[test]
fn mensaje_de_cancelacion_es_contrato() {
    assert_eq!(
        cuentacobro_lib::system::MSG_CANCELLED,
        "Operación cancelada."
    );
}

#[test]
fn el_borrado_de_datos_es_atomico_y_conserva_el_perfil_y_las_cuentas_bancarias() {
    use cuentacobro_lib::db::{clear_billing_data, init_db};
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         INSERT INTO clients (id, name) VALUES (1, 'Cliente');
         INSERT INTO bank_accounts (bank, account_type, account_number) VALUES ('Banco', 'Ahorros', '123');
         INSERT INTO invoices (id, number, date, client_id, total) VALUES (1, 1, '2026-11-18', 1, 20);
         INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (1, 'Servicio', 1, 20);
         INSERT INTO invoice_payments (invoice_id, date, amount) VALUES (1, '2026-11-18', 10);
         CREATE TRIGGER conservar_cliente BEFORE DELETE ON clients
           BEGIN SELECT RAISE(ABORT, 'cliente en uso'); END;",
    )
    .unwrap();
    let counts = |conn: &rusqlite::Connection| {
        [
            "clients",
            "invoices",
            "invoice_items",
            "invoice_payments",
            "profile",
            "bank_accounts",
        ]
        .map(|table| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap()
        })
    };
    assert!(clear_billing_data(&mut conn).is_err());
    assert_eq!(counts(&conn), [1, 1, 1, 1, 1, 1]);

    conn.execute_batch("DROP TRIGGER conservar_cliente")
        .unwrap();
    clear_billing_data(&mut conn).unwrap();
    assert_eq!(counts(&conn), [0, 0, 0, 0, 1, 1]);
}
