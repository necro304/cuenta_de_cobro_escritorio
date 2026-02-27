import Database from 'better-sqlite3'
import path from 'node:path'
import { app } from 'electron'

const dbPath = path.join(app.getPath('userData'), 'database.sqlite')
const db = new Database(dbPath)

export function initDb() {
  db.exec(`
    CREATE TABLE IF NOT EXISTS profile (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      name TEXT,
      document_id TEXT,
      address TEXT,
      phone TEXT,
      email TEXT,
      bank_info TEXT,
      signature_path TEXT
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
  `)
  
  // Migrations
  try {
    db.prepare('ALTER TABLE clients ADD COLUMN city TEXT').run()
  } catch {
    // Column might already exist
  }

  try {
    db.prepare('ALTER TABLE invoices ADD COLUMN bank_account_id INTEGER REFERENCES bank_accounts(id)').run()
  } catch {
    // Column might already exist
  }

  // Initialize profile if not exists
  const profile = db.prepare('SELECT * FROM profile WHERE id = 1').get()
  if (!profile) {
    db.prepare('INSERT INTO profile (id, name) VALUES (1, ?)').run('Mi Nombre')
  }
}

export default db
