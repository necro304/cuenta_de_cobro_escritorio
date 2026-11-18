//! Libro de cuentas y operaciones de abonos compartidas por la interfaz y la CLI.
use super::{
    get_paid_cents, get_persisted_concept_total, get_summary, is_valid_date, read_concepts,
};
use crate::{
    generic::sqlite_message,
    money::{database_number_to_cents, decimal_to_database_number, parse_decimal},
    sqlvalue::Sql,
    types::CuentaDeCobroSummary,
};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerInvoice {
    id: i64,
    number: serde_json::Value,
    date: String,
    client_name: String,
    summary: CuentaDeCobroSummary,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPayment {
    pub invoice_id: i64,
    pub date: String,
    pub amount: String,
    #[serde(default)]
    pub notes: String,
}

struct AccountBalance {
    summary: CuentaDeCobroSummary,
    valid: bool,
}

fn balance(conn: &Connection, id: i64, stored: &Sql) -> rusqlite::Result<AccountBalance> {
    let total = get_persisted_concept_total(&read_concepts(conn, id)?, stored);
    let paid = get_paid_cents(conn, id)?;
    Ok(AccountBalance {
        summary: get_summary(total.total_cents, paid.cents),
        valid: !(paid.invalid || (total.invalid_concepts && total.invalid_total)),
    })
}

pub fn ledger(conn: &mut Connection) -> rusqlite::Result<Vec<LedgerInvoice>> {
    let tx = conn.transaction()?;
    let invoices = {
        let mut st = tx.prepare(
            "SELECT i.id, i.number, i.date, c.name, i.total
             FROM invoices i JOIN clients c ON c.id = i.client_id
             ORDER BY i.date DESC, i.number DESC",
        )?;
        st.query_map([], |row| {
            let id = row.get(0)?;
            Ok(LedgerInvoice {
                id,
                number: Sql::from_ref(row.get_ref(1)?).to_json(),
                date: Sql::from_ref(row.get_ref(2)?).to_js_string(),
                client_name: row.get(3)?,
                summary: balance(&tx, id, &Sql::from_ref(row.get_ref(4)?))?.summary,
            })
        })?
        .collect::<rusqlite::Result<_>>()?
    };
    tx.commit()?;
    Ok(invoices)
}

#[derive(Serialize)]
pub struct PaymentHistory {
    summary: CuentaDeCobroSummary,
    payments: Vec<serde_json::Map<String, serde_json::Value>>,
}

fn account_balance(conn: &Connection, invoice_id: i64) -> Result<AccountBalance, String> {
    let stored = conn
        .query_row(
            "SELECT total FROM invoices WHERE id = ?",
            [invoice_id],
            |r| Ok(Sql::from_ref(r.get_ref(0)?)),
        )
        .map_err(sqlite_message)?;
    balance(conn, invoice_id, &stored).map_err(sqlite_message)
}

pub fn history(conn: &mut Connection, invoice_id: i64) -> Result<PaymentHistory, String> {
    let tx = conn.transaction().map_err(sqlite_message)?;
    let account = account_balance(&tx, invoice_id)?;
    let payments = crate::generic::db_query(
        &tx,
        "SELECT * FROM invoice_payments WHERE invoice_id = ? ORDER BY date DESC, id DESC",
        Some(vec![invoice_id.into()]),
    )?;
    tx.commit().map_err(sqlite_message)?;
    Ok(PaymentHistory {
        summary: account.summary,
        payments,
    })
}

pub(crate) fn repair_statuses(conn: &Connection) -> rusqlite::Result<()> {
    let version =
        |c: &Connection| c.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0));
    if version(conn)? >= 1 {
        return Ok(());
    }
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    if version(&tx)? == 0 {
        tx.execute_batch(
            "CREATE INDEX IF NOT EXISTS invoice_payments_by_invoice ON invoice_payments(invoice_id);
             CREATE INDEX IF NOT EXISTS invoice_items_by_invoice ON invoice_items(invoice_id);",
        )?;
        let mut repairs = Vec::new();
        {
            let mut st = tx.prepare("SELECT id, total, status FROM invoices")?;
            let mut rows = st.query([])?;
            while let Some(row) = rows.next()? {
                let id = row.get(0)?;
                let account = balance(&tx, id, &Sql::from_ref(row.get_ref(1)?))?;
                let status = account.summary.payment_status.as_str();
                if account.valid && row.get_ref(2)?.as_str().ok() != Some(status) {
                    repairs.push((id, account.summary.payment_status));
                }
            }
        }
        let mut update = tx.prepare("UPDATE invoices SET status = ?2 WHERE id = ?1")?;
        for (id, status) in repairs {
            update.execute(params![id, status.as_str()])?;
        }
        tx.pragma_update(None, "user_version", 1)?;
    }
    tx.commit()
}

fn update_balance(tx: &Transaction<'_>, invoice_id: i64) -> Result<CuentaDeCobroSummary, String> {
    let account = account_balance(tx, invoice_id)?;
    if !account.valid {
        return Err("Revisa los importes de esta cuenta y sus abonos.".into());
    }
    tx.execute(
        "UPDATE invoices SET status = ? WHERE id = ?",
        params![account.summary.payment_status.as_str(), invoice_id],
    )
    .map_err(sqlite_message)?;
    Ok(account.summary)
}

pub fn save(conn: &mut Connection, payment: NewPayment) -> Result<CuentaDeCobroSummary, String> {
    if !is_valid_date(&payment.date) {
        return Err("Ingresa una fecha de abono válida.".into());
    }
    let amount = parse_decimal(&payment.amount, false)
        .filter(|decimal| decimal.scale <= 2)
        .and_then(|decimal| decimal_to_database_number(&decimal))
        .filter(|n| database_number_to_cents(*n).is_some())
        .ok_or("Ingresa un monto positivo expresado en pesos y centavos.")?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sqlite_message)?;
    tx.execute(
        "INSERT INTO invoice_payments (invoice_id, date, amount, notes) VALUES (?, ?, ?, ?)",
        params![payment.invoice_id, payment.date, amount, payment.notes],
    )
    .map_err(sqlite_message)?;
    let summary = update_balance(&tx, payment.invoice_id)?;
    tx.commit().map_err(sqlite_message)?;
    Ok(summary)
}

pub fn delete(
    conn: &mut Connection,
    invoice_id: i64,
    payment_id: i64,
) -> Result<CuentaDeCobroSummary, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sqlite_message)?;
    let changes = tx
        .execute(
            "DELETE FROM invoice_payments WHERE invoice_id = ? AND id = ?",
            params![invoice_id, payment_id],
        )
        .map_err(sqlite_message)?;
    if changes == 0 {
        return Err("El abono ya no existe en esta cuenta.".into());
    }
    let summary = update_balance(&tx, invoice_id)?;
    tx.commit().map_err(sqlite_message)?;
    Ok(summary)
}
