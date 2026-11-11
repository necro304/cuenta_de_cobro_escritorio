//! Port de electron/cuentaDeCobro.ts.
use crate::money::*;
use crate::sqlvalue::{Sql, is_safe_integer};
use crate::types::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

type R<T> = CuentaDeCobroResult<T>;

struct NormalizedConcept {
    description: String,
    quantity_db: f64,
    price_db: f64,
}
struct NormalizedCuenta {
    number: i64,
    date: String,
    client_id: i64,
    bank_account_id: i64,
    notes: String,
    concepts: Vec<NormalizedConcept>,
    total_cents: Cents,
    total_db: f64,
}

pub fn today_local_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// /^(\d{4})-(\d{2})-(\d{2})$/ + validación con Date.UTC (años 0000-0099 son inválidos: Date.UTC los mapea a 1900-1999).
pub fn is_valid_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !b
        .iter()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = value[0..4].parse().unwrap();
    let month: u32 = value[5..7].parse().unwrap();
    let day: u32 = value[8..10].parse().unwrap();
    if year < 100 || !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let max = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= max
}

/// Igual que is_valid_date pero sobre el valor crudo de la columna (regex.exec(String(value))).
fn is_valid_date_sql(v: &Sql) -> bool {
    match v {
        Sql::Text(_) | Sql::Blob(_) => is_valid_date(&v.to_js_string()),
        _ => false,
    }
}

fn get_next_number(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    let last: Sql = conn.query_row(
        "SELECT MAX(number) AS last FROM invoices WHERE number > 0",
        [],
        |r| Ok(Sql::from_ref(r.get_ref(0)?)),
    )?;
    let next = match last {
        Sql::Null => 1.0,
        Sql::Int(_) | Sql::Real(_) => last.as_js_number().unwrap() + 1.0,
        _ => return Ok(None), // texto/blob: "abc" + 1 es un string en JS → no es entero seguro
    };
    Ok(is_safe_integer(next).then_some(next as i64))
}

fn get_clients(conn: &Connection) -> rusqlite::Result<Vec<ClienteOption>> {
    let mut st = conn.prepare("SELECT id, name FROM clients ORDER BY name COLLATE NOCASE, id")?;
    let rows = st.query_map([], |r| {
        Ok(ClienteOption {
            id: r.get(0)?,
            name: Sql::from_ref(r.get_ref(1)?).to_js_string(),
        })
    })?;
    rows.collect()
}

fn get_bank_accounts(conn: &Connection) -> rusqlite::Result<Vec<CuentaBancariaOption>> {
    let mut st = conn.prepare("SELECT id, bank, account_type, account_number, is_default FROM bank_accounts ORDER BY is_default DESC, bank COLLATE NOCASE, id")?;
    let rows = st.query_map([], |r| {
        Ok(CuentaBancariaOption {
            id: r.get(0)?,
            bank: Sql::from_ref(r.get_ref(1)?).to_js_string(),
            account_type: Sql::from_ref(r.get_ref(2)?).to_js_string(),
            account_number: Sql::from_ref(r.get_ref(3)?).to_js_string(),
            is_default: Sql::from_ref(r.get_ref(4)?).js_truthy(),
        })
    })?;
    rows.collect()
}

struct Paid {
    cents: Cents,
    invalid: bool,
}

fn get_paid_cents(conn: &Connection, invoice_id: i64) -> rusqlite::Result<Paid> {
    let mut st =
        conn.prepare("SELECT amount FROM invoice_payments WHERE invoice_id = ? ORDER BY id")?;
    let amounts: Vec<Sql> = st
        .query_map([invoice_id], |r| Ok(Sql::from_ref(r.get_ref(0)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut cents: Cents = 0;
    for amount in amounts {
        let Some(n) = amount.as_js_number() else {
            return Ok(Paid {
                cents,
                invalid: true,
            });
        };
        if n < 0.0 {
            return Ok(Paid {
                cents,
                invalid: true,
            });
        }
        match database_number_to_cents(n) {
            Some(c) if cents + c <= MAX_SAFE_CENTS => cents += c,
            _ => {
                return Ok(Paid {
                    cents,
                    invalid: true,
                });
            }
        }
    }
    Ok(Paid {
        cents,
        invalid: false,
    })
}

fn determine_payment_status(total: Cents, paid: Cents) -> EstadoPago {
    if paid == 0 {
        EstadoPago::Draft
    } else if paid < total {
        EstadoPago::PartiallyPaid
    } else {
        EstadoPago::Paid
    }
}

fn get_summary(total: Cents, paid: Cents) -> CuentaDeCobroSummary {
    CuentaDeCobroSummary {
        total: cents_to_money(total),
        paid_amount: cents_to_money(paid),
        balance: cents_to_money(total.saturating_sub(paid)),
        payment_status: determine_payment_status(total, paid),
    }
}

fn issue(field: impl Into<String>, code: ValidationCode) -> ValidationIssue {
    ValidationIssue {
        field: field.into(),
        code,
    }
}

/// `/^\d+$/.test(s.trim()) ? Number(s) : NaN` + `Number.isSafeInteger(n) && n > 0`.
pub fn parse_positive_safe_integer(s: &str) -> Option<i64> {
    let trimmed = js_trim(s);
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits = trimmed.trim_start_matches('0');
    if digits.is_empty() || digits.len() > 16 {
        return None;
    } // "000" → 0; >16 dígitos ≥ 1e16 > 2^53-1
    let n: i64 = digits.parse().ok()?;
    (n as f64 <= crate::sqlvalue::MAX_SAFE_INTEGER).then_some(n)
}

fn normalize_cuenta(c: &CuentaDeCobroDraft) -> Result<NormalizedCuenta, CuentaDeCobroError> {
    use ValidationCode::*;
    let mut issues = Vec::new();

    let number = parse_positive_safe_integer(&c.number);
    if number.is_none() {
        issues.push(issue("number", InvalidPositiveInteger));
    }
    if !is_valid_date(&c.date) {
        issues.push(issue("date", InvalidDate));
    }
    let positive_safe = |n: JsNum| {
        n.0.filter(|x| is_safe_integer(*x) && *x > 0.0)
            .map(|x| x as i64)
    };
    let client_id = positive_safe(c.client_id);
    if client_id.is_none() {
        issues.push(issue("clientId", Required));
    }
    let bank_account_id = positive_safe(c.bank_account_id);
    if bank_account_id.is_none() {
        issues.push(issue("bankAccountId", Required));
    }
    if c.concepts.is_empty() {
        issues.push(issue("concepts", AtLeastOneConceptRequired));
    }

    let mut concepts = Vec::new();
    let mut total_cents: Cents = 0;
    for (index, concept) in c.concepts.iter().enumerate() {
        let description = js_trim(&concept.description).to_string();
        let quantity = parse_decimal(&concept.quantity, false);
        let price = parse_decimal(&concept.price, false);
        let quantity_db = quantity.as_ref().and_then(decimal_to_database_number);
        let price_db = price.as_ref().and_then(decimal_to_database_number);
        if description.is_empty() {
            issues.push(issue(format!("concepts.{index}.description"), Required));
        }
        if quantity.is_none() {
            issues.push(issue(
                format!("concepts.{index}.quantity"),
                InvalidPositiveDecimal,
            ));
        }
        if price.is_none() {
            issues.push(issue(
                format!("concepts.{index}.price"),
                InvalidPositiveDecimal,
            ));
        }
        if quantity.is_some() && quantity_db.is_none() {
            issues.push(issue(
                format!("concepts.{index}.quantity"),
                AmountOutOfRange,
            ));
        }
        if price.is_some() && price_db.is_none() {
            issues.push(issue(format!("concepts.{index}.price"), AmountOutOfRange));
        }
        let (Some(q), Some(p), Some(q_db), Some(p_db)) = (&quantity, &price, quantity_db, price_db)
        else {
            continue;
        };
        if description.is_empty() {
            continue;
        }
        let Some(subtotal) = round_product_to_cents(q, p) else {
            issues.push(issue(format!("concepts.{index}.price"), AmountOutOfRange));
            continue;
        };
        if subtotal == 0 {
            issues.push(issue(
                format!("concepts.{index}.price"),
                SubtotalRoundsToZero,
            ));
            continue;
        }
        if total_cents + subtotal > MAX_SAFE_CENTS {
            issues.push(issue("concepts", AmountOutOfRange));
            continue;
        }
        total_cents += subtotal;
        concepts.push(NormalizedConcept {
            description,
            quantity_db: q_db,
            price_db: p_db,
        });
    }
    let total_db = cents_to_database_number(total_cents);
    if total_db.is_none() {
        issues.push(issue("concepts", AmountOutOfRange));
    }
    if !issues.is_empty() {
        return Err(CuentaDeCobroError::ValidationFailed { issues });
    }
    Ok(NormalizedCuenta {
        number: number.unwrap(),
        date: c.date.clone(),
        client_id: client_id.unwrap(),
        bank_account_id: bank_account_id.unwrap(),
        notes: js_trim(&c.notes).to_string(),
        concepts,
        total_cents,
        total_db: total_db.unwrap(),
    })
}

struct PersistedTotal {
    total_cents: Cents,
    invalid_concepts: bool,
    invalid_total: bool,
}

fn get_persisted_concept_total(
    concepts: &[ConceptoDeCobroDraft],
    stored_total: &Sql,
) -> PersistedTotal {
    let stored_cents = match stored_total {
        Sql::Null => None,
        v => v.as_js_number().and_then(database_number_to_cents),
    };
    let probe = CuentaDeCobroDraft {
        number: "1".into(),
        date: "2000-01-01".into(),
        client_id: JsNum(Some(1.0)),
        bank_account_id: JsNum(Some(1.0)),
        notes: String::new(),
        concepts: concepts.to_vec(),
    };
    match normalize_cuenta(&probe) {
        Err(_) => PersistedTotal {
            total_cents: stored_cents.unwrap_or(0),
            invalid_concepts: true,
            invalid_total: stored_cents.is_none(),
        },
        Ok(n) => PersistedTotal {
            total_cents: n.total_cents,
            invalid_concepts: false,
            invalid_total: stored_cents != Some(n.total_cents),
        },
    }
}

fn open_inner(
    conn: &Connection,
    target: CuentaDeCobroTarget,
) -> rusqlite::Result<R<CuentaDeCobroEditorModel>> {
    let clientes = get_clients(conn)?;
    let cuentas_bancarias = get_bank_accounts(conn)?;
    let id = match target {
        CuentaDeCobroTarget::Create => {
            let next = get_next_number(conn)?;
            let default_bank = cuentas_bancarias
                .iter()
                .find(|a| a.is_default)
                .map(|a| a.id as f64);
            return Ok(R::Ok(CuentaDeCobroEditorModel {
                target,
                cuenta: CuentaDeCobroDraft {
                    number: next.map(|n| n.to_string()).unwrap_or_default(),
                    date: today_local_date(),
                    client_id: JsNum(None),
                    bank_account_id: JsNum(default_bank),
                    notes: String::new(),
                    concepts: vec![ConceptoDeCobroDraft {
                        description: String::new(),
                        quantity: "1".into(),
                        price: String::new(),
                    }],
                },
                clientes,
                cuentas_bancarias,
                summary: get_summary(0, 0),
                diagnostics: vec![],
            }));
        }
        CuentaDeCobroTarget::Edit { id } => id,
    };
    let row = match id.as_rowid() {
        None => None,
        Some(rowid) => conn.query_row(
            "SELECT id, number, date, client_id, bank_account_id, total, notes FROM invoices WHERE id = ?",
            [rowid],
            |r| Ok((r.get::<_, i64>(0)?, (1..7).map(|i| Sql::from_ref(r.get_ref(i).unwrap())).collect::<Vec<_>>())),
        ).optional()?,
    };
    let Some((invoice_id, cols)) = row else {
        return Ok(R::Err(CuentaDeCobroError::CuentaDeCobroNotFound { id }));
    };
    let [number, date, client_id, bank_account_id, total, notes]: [Sql; 6] =
        cols.try_into().unwrap();

    let mut st = conn.prepare(
        "SELECT description, quantity, price FROM invoice_items WHERE invoice_id = ? ORDER BY id",
    )?;
    let concepts: Vec<ConceptoDeCobroDraft> = st
        .query_map([invoice_id], |r| {
            let to_decimal_text = |v: Sql| match v {
                Sql::Null => String::new(),
                v => v
                    .as_js_number()
                    .and_then(database_number_to_decimal)
                    .map(|d| d.canonical)
                    .unwrap_or_default(),
            };
            Ok(ConceptoDeCobroDraft {
                description: Sql::from_ref(r.get_ref(0)?).to_js_string(),
                quantity: to_decimal_text(Sql::from_ref(r.get_ref(1)?)),
                price: to_decimal_text(Sql::from_ref(r.get_ref(2)?)),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut diagnostics = Vec::new();
    let number_ok = number
        .as_js_number()
        .is_some_and(|n| is_safe_integer(n) && n > 0.0);
    let total_ok = total
        .as_js_number()
        .and_then(database_number_to_cents)
        .is_some();
    if !number_ok || !is_valid_date_sql(&date) || bank_account_id == Sql::Null || !total_ok {
        diagnostics.push(CuentaDeCobroDiagnostic::InvalidPersistedAccount);
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) AS count FROM invoices WHERE number = ?",
        [&number],
        |r| r.get(0),
    )?;
    if count > 1 {
        diagnostics.push(CuentaDeCobroDiagnostic::HistoricNumberConflict {
            number: number.to_json(),
        });
    }
    let matches_id = |v: &Sql, id: i64| matches!(v.as_js_number(), Some(n) if n == id as f64);
    let client_exists = clientes.iter().any(|c| matches_id(&client_id, c.id));
    if !client_exists {
        diagnostics.push(CuentaDeCobroDiagnostic::MissingClientReference {
            client_id: client_id.to_json(),
        });
    }
    let bank_exists = cuentas_bancarias
        .iter()
        .any(|a| matches_id(&bank_account_id, a.id));
    if !bank_exists {
        diagnostics.push(CuentaDeCobroDiagnostic::MissingBankAccountReference {
            bank_account_id: bank_account_id.to_json(),
        });
    }
    let calculated = get_persisted_concept_total(&concepts, &total);
    if calculated.invalid_total
        && !diagnostics.contains(&CuentaDeCobroDiagnostic::InvalidPersistedAccount)
    {
        diagnostics.push(CuentaDeCobroDiagnostic::InvalidPersistedAccount);
    }
    if calculated.invalid_concepts {
        diagnostics.push(CuentaDeCobroDiagnostic::InvalidPersistedConcepts);
    }
    let paid = get_paid_cents(conn, invoice_id)?;
    if paid.invalid {
        diagnostics.push(CuentaDeCobroDiagnostic::InvalidPersistedPayments);
    }

    Ok(R::Ok(CuentaDeCobroEditorModel {
        target: CuentaDeCobroTarget::Edit { id },
        cuenta: CuentaDeCobroDraft {
            number: number.to_js_string(),
            date: date.to_js_string(),
            client_id: JsNum(if client_exists {
                client_id.as_js_number()
            } else {
                None
            }),
            bank_account_id: JsNum(if bank_exists {
                bank_account_id.as_js_number()
            } else {
                None
            }),
            notes: match notes {
                Sql::Null => String::new(),
                v => v.to_js_string(),
            },
            concepts,
        },
        clientes,
        cuentas_bancarias,
        summary: get_summary(calculated.total_cents, paid.cents),
        diagnostics,
    }))
}

pub fn open(conn: &Connection, target: CuentaDeCobroTarget) -> R<CuentaDeCobroEditorModel> {
    open_inner(conn, target).unwrap_or(R::Err(CuentaDeCobroError::StorageFailure))
}

fn save_transaction(
    conn: &mut Connection,
    edit_id: Option<JsNum>,
    n: &NormalizedCuenta,
) -> rusqlite::Result<R<SaveCuentaDeCobroReceipt>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut existing_id: Option<i64> = None;
    if let Some(id) = edit_id {
        existing_id = match id.as_rowid() {
            Some(r) => tx
                .query_row("SELECT id FROM invoices WHERE id = ?", [r], |r| r.get(0))
                .optional()?,
            None => None,
        };
        if existing_id.is_none() {
            return Ok(R::Err(CuentaDeCobroError::CuentaDeCobroNotFound { id }));
        }
    }
    let client: Option<i64> = tx
        .query_row("SELECT id FROM clients WHERE id = ?", [n.client_id], |r| {
            r.get(0)
        })
        .optional()?;
    if client.is_none() {
        return Ok(R::Err(CuentaDeCobroError::ClienteNotFound {
            id: n.client_id,
        }));
    }
    let bank: Option<i64> = tx
        .query_row(
            "SELECT id FROM bank_accounts WHERE id = ?",
            [n.bank_account_id],
            |r| r.get(0),
        )
        .optional()?;
    if bank.is_none() {
        return Ok(R::Err(CuentaDeCobroError::CuentaBancariaNotFound {
            id: n.bank_account_id,
        }));
    }
    let duplicate: Option<i64> = match existing_id {
        Some(id) => tx
            .query_row(
                "SELECT id FROM invoices WHERE number = ? AND id <> ? LIMIT 1",
                [n.number, id],
                |r| r.get(0),
            )
            .optional()?,
        None => tx
            .query_row(
                "SELECT id FROM invoices WHERE number = ? LIMIT 1",
                [n.number],
                |r| r.get(0),
            )
            .optional()?,
    };
    if duplicate.is_some() {
        return Ok(R::Err(CuentaDeCobroError::NumberInUse {
            number: n.number,
            suggested_number: get_next_number(&tx)?,
        }));
    }
    let paid = match existing_id {
        Some(id) => get_paid_cents(&tx, id)?,
        None => Paid {
            cents: 0,
            invalid: false,
        },
    };
    if paid.invalid {
        return Ok(R::Err(CuentaDeCobroError::StorageFailure));
    }
    if n.total_cents < paid.cents {
        return Ok(R::Err(CuentaDeCobroError::TotalBelowPaidAmount {
            total: cents_to_money(n.total_cents),
            paid_amount: cents_to_money(paid.cents),
        }));
    }
    let status = determine_payment_status(n.total_cents, paid.cents);
    let invoice_id = match existing_id {
        Some(id) => {
            tx.execute("UPDATE invoices SET number = ?, date = ?, client_id = ?, bank_account_id = ?, total = ?, notes = ?, status = ? WHERE id = ?",
                params![n.number, n.date, n.client_id, n.bank_account_id, n.total_db, n.notes, status.as_str(), id])?;
            tx.execute("DELETE FROM invoice_items WHERE invoice_id = ?", [id])?;
            id
        }
        None => {
            tx.execute("INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![n.number, n.date, n.client_id, n.bank_account_id, n.total_db, n.notes, status.as_str()])?;
            tx.last_insert_rowid()
        }
    };
    {
        let mut insert = tx.prepare("INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)")?;
        for c in &n.concepts {
            insert.execute(params![
                invoice_id,
                c.description,
                c.quantity_db,
                c.price_db
            ])?;
        }
    }
    tx.commit()?;
    Ok(R::Ok(SaveCuentaDeCobroReceipt {
        id: invoice_id,
        number: n.number,
        total: cents_to_money(n.total_cents),
        status,
    }))
}

/// `conn = None` modela la conexión cerrada: la validación corre igual (en JS ocurre fuera del try).
pub fn save(
    conn: Option<&mut Connection>,
    command: SaveCuentaDeCobroCommand,
) -> R<SaveCuentaDeCobroReceipt> {
    let (edit_id, cuenta) = match command {
        SaveCuentaDeCobroCommand::Create { cuenta } => (None, cuenta),
        SaveCuentaDeCobroCommand::Edit { id, cuenta } => (Some(id), cuenta),
    };
    let normalized = match normalize_cuenta(&cuenta) {
        Ok(n) => n,
        Err(e) => return R::Err(e),
    };
    let Some(conn) = conn else {
        return R::Err(CuentaDeCobroError::StorageFailure);
    };
    save_transaction(conn, edit_id, &normalized)
        .unwrap_or(R::Err(CuentaDeCobroError::StorageFailure))
}
