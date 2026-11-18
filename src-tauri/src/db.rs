use rusqlite::{Connection, OptionalExtension};

pub const SCHEMA_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS profile (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      name TEXT,
      document_type TEXT,
      document_id TEXT,
      rut TEXT,
      address TEXT,
      phone TEXT,
      email TEXT,
      bank_info TEXT,
      signature_path TEXT,
      signature TEXT,
      signature_color TEXT DEFAULT '#245f46',
      signature_mode TEXT DEFAULT 'auto',
      default_template TEXT DEFAULT 'default'
    );

    CREATE TABLE IF NOT EXISTS clients (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL,
      document_id TEXT,
      address TEXT,
      city TEXT,
      phone TEXT,
      email TEXT,
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );

    CREATE TABLE IF NOT EXISTS invoices (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      number INTEGER NOT NULL,
      date DATE NOT NULL,
      client_id INTEGER NOT NULL,
      total DECIMAL(10, 2) DEFAULT 0,
      notes TEXT,
      status TEXT DEFAULT 'draft',
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (client_id) REFERENCES clients(id)
    );

    CREATE TABLE IF NOT EXISTS invoice_items (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      invoice_id INTEGER NOT NULL,
      description TEXT NOT NULL,
      quantity DECIMAL(10, 2) NOT NULL,
      price DECIMAL(10, 2) NOT NULL,
      FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS invoice_payments (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      invoice_id INTEGER NOT NULL,
      date DATE NOT NULL,
      amount DECIMAL(10, 2) NOT NULL,
      notes TEXT,
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS bank_accounts (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      bank TEXT NOT NULL,
      account_type TEXT NOT NULL,
      account_number TEXT NOT NULL,
      is_default INTEGER DEFAULT 0,
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );
"#;

pub const MIGRATIONS: &[&str] = &[
    "ALTER TABLE clients ADD COLUMN city TEXT",
    "ALTER TABLE invoices ADD COLUMN bank_account_id INTEGER REFERENCES bank_accounts(id)",
    "ALTER TABLE profile ADD COLUMN document_type TEXT",
    "ALTER TABLE profile ADD COLUMN rut TEXT",
    "ALTER TABLE profile ADD COLUMN signature TEXT",
    "ALTER TABLE profile ADD COLUMN signature_color TEXT DEFAULT '#245f46'",
    "ALTER TABLE profile ADD COLUMN signature_mode TEXT DEFAULT 'auto'",
    "ALTER TABLE profile ADD COLUMN default_template TEXT DEFAULT 'default'",
];

/// Equivale a `new Database(path)` de better-sqlite3 12: FK activas, busy timeout 5 s y comillas
/// dobles solo como identificadores (SQLITE_DQS=0).
///
/// Además fija journal_mode=DELETE: el respaldo es una copia del archivo principal, así que la BD
/// nunca debe quedar en WAL (p. ej. tras restaurar un archivo editado con otra herramienta). Si ya
/// está en DELETE, no hace nada; si estaba en WAL, consolida el `-wal` y vuelve a DELETE. Nunca es
/// fatal: si otro proceso la tiene abierta en WAL y SQLite no puede cambiar el modo, se registra y
/// se sigue con la conexión abierta.
pub fn open_connection(path: &std::path::Path) -> rusqlite::Result<Connection> {
    use rusqlite::config::DbConfig;
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DQS_DML, false)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DQS_DDL, false)?;
    match conn.pragma_update_and_check(None, "journal_mode", "DELETE", |row| {
        row.get::<_, String>(0)
    }) {
        Ok(mode) if mode.eq_ignore_ascii_case("delete") => {}
        Ok(mode) => {
            log::warn!("journal_mode quedó en {mode}: el respaldo podría no incluir el -wal")
        }
        Err(e) => log::warn!("No se pudo fijar journal_mode=DELETE: {e}"),
    }
    Ok(conn)
}

pub fn clear_billing_data(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    // El borrado explícito también retira posibles registros huérfanos de bases antiguas.
    tx.execute_batch(
        "DELETE FROM invoice_payments;
         DELETE FROM invoice_items;
         DELETE FROM invoices;
         DELETE FROM clients;",
    )?;
    tx.commit()
}

pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    for sql in MIGRATIONS {
        let _ = conn.execute(sql, []);
    }
    let exists = conn
        .query_row("SELECT * FROM profile WHERE id = 1", [], |_| Ok(()))
        .optional()?;
    if exists.is_none() {
        conn.execute(
            "INSERT INTO profile (id, name) VALUES (1, ?)",
            ["Mi Nombre"],
        )?;
    }
    crate::cuenta_de_cobro::abonos::repair_statuses(conn)
}
