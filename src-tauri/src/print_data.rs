use crate::{Database, commands::with_conn, generic};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use tauri::State;

type Record = Map<String, Value>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoicePrintData {
    profile: Record,
    client: Record,
    invoice: Record,
    items: Vec<Record>,
    bank_account: Option<Record>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReportData {
    profile: Record,
    client: Record,
    invoices: Vec<Record>,
    monthly_invoices: Vec<Record>,
    monthly_payments: Vec<Record>,
    payments: Vec<Record>,
}

fn invoice_print_data(conn: &mut Connection, id: i64) -> Result<InvoicePrintData, String> {
    let tx = conn.transaction().map_err(generic::sqlite_message)?;
    let invoice = generic::db_get(
        &tx,
        "SELECT id, number, date, total, client_id, bank_account_id FROM invoices WHERE id = ?",
        Some(vec![id.into()]),
    )?
    .ok_or("No se encontró la cuenta de cobro.")?;
    let client = generic::db_get(
        &tx,
        "SELECT name, document_id, address, city, phone FROM clients WHERE id = ?",
        Some(vec![invoice["client_id"].clone()]),
    )?
    .ok_or("No se encontró el cliente de la cuenta.")?;
    let bank_account = if invoice["bank_account_id"].is_null() {
        None
    } else {
        Some(
            generic::db_get(
                &tx,
                "SELECT id, bank, account_type, account_number, is_default FROM bank_accounts WHERE id = ?",
                Some(vec![invoice["bank_account_id"].clone()]),
            )?
            .ok_or("No se encontró la cuenta bancaria.")?,
        )
    };
    let profile = generic::db_get(
        &tx,
        "SELECT id, name, document_type, document_id, rut, address, phone, email, bank_info,
                signature, signature_color, signature_mode, default_template FROM profile WHERE id = 1",
        None,
    )?
    .ok_or("No se encontró el perfil.")?;
    let items = generic::db_query(
        &tx,
        "SELECT id, invoice_id, description, quantity, price FROM invoice_items WHERE invoice_id = ? ORDER BY id",
        Some(vec![id.into()]),
    )?;
    tx.commit().map_err(generic::sqlite_message)?;
    Ok(InvoicePrintData {
        profile,
        client,
        invoice,
        items,
        bank_account,
    })
}

fn client_report_data(conn: &mut Connection, id: i64) -> Result<ClientReportData, String> {
    let tx = conn.transaction().map_err(generic::sqlite_message)?;
    let client = generic::db_get(
        &tx,
        "SELECT name, document_id, city, phone, email FROM clients WHERE id = ?",
        Some(vec![id.into()]),
    )?
    .ok_or("No se encontró el cliente.")?;
    let profile = generic::db_get(&tx, "SELECT name FROM profile WHERE id = 1", None)?
        .ok_or("No se encontró el perfil.")?;
    let invoices = generic::db_query(
        &tx,
        "SELECT i.id, i.number, i.date, i.status, i.total,
                COALESCE(SUM(p.amount), 0) AS paid_amount,
                MAX(i.total - COALESCE(SUM(p.amount), 0), 0) AS balance
         FROM invoices i LEFT JOIN invoice_payments p ON p.invoice_id = i.id
         WHERE i.client_id = ? GROUP BY i.id ORDER BY i.date DESC, i.number DESC",
        Some(vec![id.into()]),
    )?;
    let monthly_invoices = generic::db_query(
        &tx,
        "SELECT strftime('%Y-%m', date) AS month, COALESCE(SUM(total), 0) AS total FROM invoices
         WHERE client_id = ? AND date >= date('now', 'localtime', 'start of month', '-5 months')
         GROUP BY month ORDER BY month",
        Some(vec![id.into()]),
    )?;
    let monthly_payments = generic::db_query(
        &tx,
        "SELECT strftime('%Y-%m', p.date) AS month, COALESCE(SUM(p.amount), 0) AS total
         FROM invoice_payments p JOIN invoices i ON i.id = p.invoice_id
         WHERE i.client_id = ? AND p.date >= date('now', 'localtime', 'start of month', '-5 months')
         GROUP BY month ORDER BY month",
        Some(vec![id.into()]),
    )?;
    let payments = generic::db_query(
        &tx,
        "SELECT p.id, p.date, p.amount, p.notes, i.number AS invoice_number
         FROM invoice_payments p JOIN invoices i ON i.id = p.invoice_id
         WHERE i.client_id = ? ORDER BY p.date DESC, p.id DESC LIMIT 8",
        Some(vec![id.into()]),
    )?;
    tx.commit().map_err(generic::sqlite_message)?;
    Ok(ClientReportData {
        profile,
        client,
        invoices,
        monthly_invoices,
        monthly_payments,
        payments,
    })
}

#[tauri::command]
pub async fn read_invoice_print_data(
    db: State<'_, Database>,
    invoice_id: i64,
) -> Result<InvoicePrintData, String> {
    with_conn(&db, move |conn| invoice_print_data(conn, invoice_id)).await
}

#[tauri::command]
pub async fn read_client_report_data(
    db: State<'_, Database>,
    client_id: i64,
) -> Result<ClientReportData, String> {
    with_conn(&db, move |conn| client_report_data(conn, client_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO clients (id, name) VALUES (1, 'Mar'), (2, 'Sol');
             INSERT INTO bank_accounts (id, bank, account_type, account_number)
                VALUES (9, 'Banco', 'Ahorros', '123');
             INSERT INTO invoices (id, number, date, client_id, total, bank_account_id)
                VALUES (3, 24, date('now', 'localtime'), 1, 150, 9),
                       (4, 25, date('now', 'localtime'), 2, 200, NULL);
             INSERT INTO invoice_items (invoice_id, description, quantity, price)
                VALUES (3, 'Consulta', 1, 150), (4, 'Servicio', 1, 200);
             INSERT INTO invoice_payments (invoice_id, date, amount)
                VALUES (3, date('now', 'localtime'), 60), (4, date('now', 'localtime'), 200);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn lee_la_cuenta_con_su_cliente_perfil_banco_y_conceptos() {
        let mut conn = database();
        let data = invoice_print_data(&mut conn, 3).unwrap();
        assert_eq!(data.invoice["number"], 24);
        assert_eq!(data.client["name"], "Mar");
        assert_eq!(data.profile["name"], "Mi Nombre");
        assert_eq!(data.bank_account.unwrap()["account_number"], "123");
        assert_eq!(data.items.len(), 1);
        assert_eq!(data.items[0]["description"], "Consulta");
        assert!(
            invoice_print_data(&mut conn, 4)
                .unwrap()
                .bank_account
                .is_none()
        );
    }

    #[test]
    fn informe_solo_incluye_movimientos_del_cliente_y_nombre_del_emisor() {
        let data = client_report_data(&mut database(), 1).unwrap();
        assert_eq!(data.client["name"], "Mar");
        assert_eq!(data.profile.len(), 1);
        assert_eq!(data.profile["name"], "Mi Nombre");
        assert_eq!(data.invoices.len(), 1);
        assert_eq!(data.invoices[0]["paid_amount"], 60);
        assert_eq!(data.invoices[0]["balance"], 90);
        assert_eq!(data.monthly_invoices[0]["total"], 150);
        assert_eq!(data.monthly_payments[0]["total"], 60);
        assert_eq!(data.payments.len(), 1);
        assert_eq!(data.payments[0]["invoice_number"], 24);
    }

    #[test]
    fn una_fila_ausente_o_un_error_de_lectura_impiden_entregar_datos_parciales() {
        let mut conn = database();
        assert!(invoice_print_data(&mut conn, 99).is_err());
        assert!(client_report_data(&mut conn, 99).is_err());
        conn.execute_batch("DROP TABLE invoice_items; DROP TABLE invoice_payments;")
            .unwrap();
        assert!(invoice_print_data(&mut conn, 3).is_err());
        assert!(client_report_data(&mut conn, 1).is_err());
    }
}
