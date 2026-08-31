import db from './database'
import {
  MAX_SAFE_CENTS,
  centsToDatabaseNumber,
  centsToMoney,
  databaseNumberToCents,
  databaseNumberToDecimal,
  decimalToDatabaseNumber,
  parseDecimal,
  roundProductToCents,
} from '../src/lib/cuentaDeCobroMoney'
import type {
  ClienteOption,
  ConceptoDeCobroDraft,
  CuentaBancariaOption,
  CuentaDeCobroDiagnostic,
  CuentaDeCobroDraft,
  CuentaDeCobroModule,
  CuentaDeCobroResult,
  CuentaDeCobroValidationIssue,
  EstadoPago,
  SaveCuentaDeCobroCommand,
  SaveCuentaDeCobroReceipt,
} from '../src/types/cuentaDeCobro'

interface InvoiceRow {
  id: number
  number: number
  date: string
  client_id: number
  bank_account_id: number | null
  total: number | null
  notes: string | null
}

interface InvoiceItemRow {
  description: string
  quantity: number | null
  price: number | null
}

interface ClientRow {
  id: number
  name: string
}

interface BankAccountRow {
  id: number
  bank: string
  account_type: string
  account_number: string
  is_default: number
}

interface PaymentRow {
  amount: number | null
}

interface NormalizedConcept {
  description: string
  quantity: string
  price: string
  quantityDatabase: number
  priceDatabase: number
  subtotalCents: bigint
}

interface NormalizedCuenta {
  number: number
  date: string
  clientId: number
  bankAccountId: number
  notes: string
  concepts: NormalizedConcept[]
  totalCents: bigint
  totalDatabase: number
}

const success = <T>(value: T): CuentaDeCobroResult<T> => ({ ok: true, value })

const storageFailure = <T>(): CuentaDeCobroResult<T> => ({
  ok: false,
  error: { code: 'STORAGE_FAILURE' },
})

const getTodayLocalDate = (): string => {
  const today = new Date()
  const year = today.getFullYear()
  const month = (today.getMonth() + 1).toString().padStart(2, '0')
  const day = today.getDate().toString().padStart(2, '0')
  return `${year}-${month}-${day}`
}

const isValidDate = (value: string): boolean => {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value)
  if (!match) return false

  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  const date = new Date(Date.UTC(year, month - 1, day))

  return (
    date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
  )
}

const getNextNumber = (): number | null => {
  const row = db.prepare('SELECT MAX(number) AS last FROM invoices WHERE number > 0').get() as {
    last: number | null
  }
  const next = (row.last ?? 0) + 1
  return Number.isSafeInteger(next) ? next : null
}

const getClients = (): ClienteOption[] => {
  const rows = db
    .prepare('SELECT id, name FROM clients ORDER BY name COLLATE NOCASE, id')
    .all() as ClientRow[]
  return rows.map((row) => ({ id: row.id, name: row.name }))
}

const getBankAccounts = (): CuentaBancariaOption[] => {
  const rows = db
    .prepare(
      'SELECT id, bank, account_type, account_number, is_default FROM bank_accounts ORDER BY is_default DESC, bank COLLATE NOCASE, id',
    )
    .all() as BankAccountRow[]

  return rows.map((row) => ({
    id: row.id,
    bank: row.bank,
    accountType: row.account_type,
    accountNumber: row.account_number,
    isDefault: Boolean(row.is_default),
  }))
}

const getPaidCents = (invoiceId: number): { cents: bigint; invalid: boolean } => {
  const payments = db
    .prepare('SELECT amount FROM invoice_payments WHERE invoice_id = ? ORDER BY id')
    .all(invoiceId) as PaymentRow[]

  let cents = 0n
  for (const payment of payments) {
    if (payment.amount === null || payment.amount < 0) return { cents, invalid: true }

    const paymentCents = databaseNumberToCents(payment.amount)
    if (paymentCents === null || cents + paymentCents > MAX_SAFE_CENTS) {
      return { cents, invalid: true }
    }
    cents += paymentCents
  }

  return { cents, invalid: false }
}

const determinePaymentStatus = (totalCents: bigint, paidCents: bigint): EstadoPago => {
  if (paidCents === 0n) return 'draft'
  if (paidCents < totalCents) return 'partially_paid'
  return 'paid'
}

const getSummary = (totalCents: bigint, paidCents: bigint) => ({
  total: centsToMoney(totalCents),
  paidAmount: centsToMoney(paidCents),
  balance: centsToMoney(totalCents > paidCents ? totalCents - paidCents : 0n),
  paymentStatus: determinePaymentStatus(totalCents, paidCents),
})

const normalizeCuenta = (cuenta: CuentaDeCobroDraft): CuentaDeCobroResult<NormalizedCuenta> => {
  const issues: CuentaDeCobroValidationIssue[] = []
  const number = /^\d+$/.test(cuenta.number.trim()) ? Number(cuenta.number) : Number.NaN

  if (!Number.isSafeInteger(number) || number <= 0) {
    issues.push({ field: 'number', code: 'INVALID_POSITIVE_INTEGER' })
  }
  if (!isValidDate(cuenta.date)) issues.push({ field: 'date', code: 'INVALID_DATE' })
  if (!Number.isSafeInteger(cuenta.clientId) || (cuenta.clientId ?? 0) <= 0) {
    issues.push({ field: 'clientId', code: 'REQUIRED' })
  }
  if (!Number.isSafeInteger(cuenta.bankAccountId) || (cuenta.bankAccountId ?? 0) <= 0) {
    issues.push({ field: 'bankAccountId', code: 'REQUIRED' })
  }
  if (cuenta.concepts.length === 0) {
    issues.push({ field: 'concepts', code: 'AT_LEAST_ONE_CONCEPT_REQUIRED' })
  }

  const concepts: NormalizedConcept[] = []
  let totalCents = 0n

  cuenta.concepts.forEach((concept, index) => {
    const description = concept.description.trim()
    const quantity = parseDecimal(concept.quantity, false)
    const price = parseDecimal(concept.price, false)
    const quantityDatabase = quantity ? decimalToDatabaseNumber(quantity) : null
    const priceDatabase = price ? decimalToDatabaseNumber(price) : null

    if (!description) {
      issues.push({ field: `concepts.${index}.description`, code: 'REQUIRED' })
    }
    if (!quantity) {
      issues.push({ field: `concepts.${index}.quantity`, code: 'INVALID_POSITIVE_DECIMAL' })
    }
    if (!price) {
      issues.push({ field: `concepts.${index}.price`, code: 'INVALID_POSITIVE_DECIMAL' })
    }
    if (quantity && quantityDatabase === null) {
      issues.push({ field: `concepts.${index}.quantity`, code: 'AMOUNT_OUT_OF_RANGE' })
    }
    if (price && priceDatabase === null) {
      issues.push({ field: `concepts.${index}.price`, code: 'AMOUNT_OUT_OF_RANGE' })
    }
    if (!description || !quantity || !price || quantityDatabase === null || priceDatabase === null)
      return

    const subtotalCents = roundProductToCents(quantity, price)
    if (subtotalCents === null) {
      issues.push({ field: `concepts.${index}.price`, code: 'AMOUNT_OUT_OF_RANGE' })
      return
    }
    if (subtotalCents === 0n) {
      issues.push({ field: `concepts.${index}.price`, code: 'SUBTOTAL_ROUNDS_TO_ZERO' })
      return
    }
    if (totalCents + subtotalCents > MAX_SAFE_CENTS) {
      issues.push({ field: 'concepts', code: 'AMOUNT_OUT_OF_RANGE' })
      return
    }

    totalCents += subtotalCents
    concepts.push({
      description,
      quantity: quantity.canonical,
      price: price.canonical,
      quantityDatabase,
      priceDatabase,
      subtotalCents,
    })
  })

  const totalDatabase = centsToDatabaseNumber(totalCents)
  if (totalDatabase === null) {
    issues.push({ field: 'concepts', code: 'AMOUNT_OUT_OF_RANGE' })
  }

  if (issues.length > 0) {
    return { ok: false, error: { code: 'VALIDATION_FAILED', issues } }
  }

  return success({
    number,
    date: cuenta.date,
    clientId: cuenta.clientId as number,
    bankAccountId: cuenta.bankAccountId as number,
    notes: cuenta.notes.trim(),
    concepts,
    totalCents,
    totalDatabase: totalDatabase as number,
  })
}

const getPersistedConceptTotal = (
  concepts: ConceptoDeCobroDraft[],
  storedTotal: number | null,
): { totalCents: bigint; invalidConcepts: boolean; invalidTotal: boolean } => {
  const storedCents = storedTotal === null ? null : databaseNumberToCents(storedTotal)
  const normalized = normalizeCuenta({
    number: '1',
    date: '2000-01-01',
    clientId: 1,
    bankAccountId: 1,
    notes: '',
    concepts,
  })

  if (!normalized.ok) {
    return {
      totalCents: storedCents ?? 0n,
      invalidConcepts: true,
      invalidTotal: storedCents === null,
    }
  }
  return {
    totalCents: normalized.value.totalCents,
    invalidConcepts: false,
    invalidTotal: storedCents === null || storedCents !== normalized.value.totalCents,
  }
}

const saveTransaction = db.transaction(
  (
    command: SaveCuentaDeCobroCommand,
    normalized: NormalizedCuenta,
  ): CuentaDeCobroResult<SaveCuentaDeCobroReceipt> => {
    if (command.kind === 'edit') {
      const existing = db.prepare('SELECT id FROM invoices WHERE id = ?').get(command.id)
      if (!existing) {
        return { ok: false, error: { code: 'CUENTA_DE_COBRO_NOT_FOUND', id: command.id } }
      }
    }

    const client = db.prepare('SELECT id FROM clients WHERE id = ?').get(normalized.clientId)
    if (!client) {
      return { ok: false, error: { code: 'CLIENTE_NOT_FOUND', id: normalized.clientId } }
    }

    const bankAccount = db
      .prepare('SELECT id FROM bank_accounts WHERE id = ?')
      .get(normalized.bankAccountId)
    if (!bankAccount) {
      return {
        ok: false,
        error: { code: 'CUENTA_BANCARIA_NOT_FOUND', id: normalized.bankAccountId },
      }
    }

    const duplicate = db
      .prepare(
        command.kind === 'edit'
          ? 'SELECT id FROM invoices WHERE number = ? AND id <> ? LIMIT 1'
          : 'SELECT id FROM invoices WHERE number = ? LIMIT 1',
      )
      .get(...(command.kind === 'edit' ? [normalized.number, command.id] : [normalized.number]))

    if (duplicate) {
      return {
        ok: false,
        error: {
          code: 'NUMBER_IN_USE',
          number: normalized.number,
          suggestedNumber: getNextNumber(),
        },
      }
    }

    const paid = command.kind === 'edit' ? getPaidCents(command.id) : { cents: 0n, invalid: false }
    if (paid.invalid) return storageFailure()

    if (normalized.totalCents < paid.cents) {
      return {
        ok: false,
        error: {
          code: 'TOTAL_BELOW_PAID_AMOUNT',
          total: centsToMoney(normalized.totalCents),
          paidAmount: centsToMoney(paid.cents),
        },
      }
    }

    const paymentStatus = determinePaymentStatus(normalized.totalCents, paid.cents)
    let invoiceId: number

    if (command.kind === 'edit') {
      invoiceId = command.id
      db.prepare(
        'UPDATE invoices SET number = ?, date = ?, client_id = ?, bank_account_id = ?, total = ?, notes = ?, status = ? WHERE id = ?',
      ).run([
        normalized.number,
        normalized.date,
        normalized.clientId,
        normalized.bankAccountId,
        normalized.totalDatabase,
        normalized.notes,
        paymentStatus,
        invoiceId,
      ])
      db.prepare('DELETE FROM invoice_items WHERE invoice_id = ?').run(invoiceId)
    } else {
      const result = db
        .prepare(
          'INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, ?, ?, ?, ?, ?, ?)',
        )
        .run([
          normalized.number,
          normalized.date,
          normalized.clientId,
          normalized.bankAccountId,
          normalized.totalDatabase,
          normalized.notes,
          paymentStatus,
        ])
      invoiceId = Number(result.lastInsertRowid)
    }

    const insertConcept = db.prepare(
      'INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)',
    )
    for (const concept of normalized.concepts) {
      insertConcept.run([
        invoiceId,
        concept.description,
        concept.quantityDatabase,
        concept.priceDatabase,
      ])
    }

    return success({
      id: invoiceId,
      number: normalized.number,
      total: centsToMoney(normalized.totalCents),
      status: paymentStatus,
    })
  },
)

export const cuentaDeCobroModule: CuentaDeCobroModule = {
  async open(target) {
    try {
      const clientes = getClients()
      const cuentasBancarias = getBankAccounts()

      if (target.kind === 'create') {
        const nextNumber = getNextNumber()
        const defaultBankAccount = cuentasBancarias.find((account) => account.isDefault)

        return success({
          target,
          cuenta: {
            number: nextNumber?.toString() ?? '',
            date: getTodayLocalDate(),
            clientId: null,
            bankAccountId: defaultBankAccount?.id ?? null,
            notes: '',
            concepts: [{ description: '', quantity: '1', price: '' }],
          },
          clientes,
          cuentasBancarias,
          summary: getSummary(0n, 0n),
          diagnostics: [],
        })
      }

      const invoice = db
        .prepare(
          'SELECT id, number, date, client_id, bank_account_id, total, notes FROM invoices WHERE id = ?',
        )
        .get(target.id) as InvoiceRow | undefined

      if (!invoice) {
        return { ok: false, error: { code: 'CUENTA_DE_COBRO_NOT_FOUND', id: target.id } }
      }

      const rows = db
        .prepare(
          'SELECT description, quantity, price FROM invoice_items WHERE invoice_id = ? ORDER BY id',
        )
        .all(target.id) as InvoiceItemRow[]
      const concepts = rows.map((row) => ({
        description: row.description,
        quantity:
          row.quantity === null ? '' : (databaseNumberToDecimal(row.quantity)?.canonical ?? ''),
        price: row.price === null ? '' : (databaseNumberToDecimal(row.price)?.canonical ?? ''),
      }))
      const diagnostics: CuentaDeCobroDiagnostic[] = []

      if (
        !Number.isSafeInteger(invoice.number) ||
        invoice.number <= 0 ||
        !isValidDate(invoice.date) ||
        invoice.bank_account_id === null ||
        invoice.total === null ||
        databaseNumberToCents(invoice.total) === null
      ) {
        diagnostics.push({ code: 'INVALID_PERSISTED_ACCOUNT' })
      }

      const duplicate = db
        .prepare('SELECT COUNT(*) AS count FROM invoices WHERE number = ?')
        .get(invoice.number) as { count: number }
      if (duplicate.count > 1) {
        diagnostics.push({ code: 'HISTORIC_NUMBER_CONFLICT', number: invoice.number })
      }

      const clientExists = clientes.some((client) => client.id === invoice.client_id)
      if (!clientExists) {
        diagnostics.push({ code: 'MISSING_CLIENT_REFERENCE', clientId: invoice.client_id })
      }

      const bankAccountExists = cuentasBancarias.some(
        (account) => account.id === invoice.bank_account_id,
      )
      if (!bankAccountExists) {
        diagnostics.push({
          code: 'MISSING_BANK_ACCOUNT_REFERENCE',
          bankAccountId: invoice.bank_account_id,
        })
      }

      const calculated = getPersistedConceptTotal(concepts, invoice.total)
      if (
        calculated.invalidTotal &&
        !diagnostics.some((diagnostic) => diagnostic.code === 'INVALID_PERSISTED_ACCOUNT')
      ) {
        diagnostics.push({ code: 'INVALID_PERSISTED_ACCOUNT' })
      }
      if (calculated.invalidConcepts) {
        diagnostics.push({ code: 'INVALID_PERSISTED_CONCEPTS' })
      }
      const paid = getPaidCents(invoice.id)
      if (paid.invalid) diagnostics.push({ code: 'INVALID_PERSISTED_PAYMENTS' })

      return success({
        target,
        cuenta: {
          number: String(invoice.number),
          date: invoice.date,
          clientId: clientExists ? invoice.client_id : null,
          bankAccountId: bankAccountExists ? invoice.bank_account_id : null,
          notes: invoice.notes ?? '',
          concepts,
        },
        clientes,
        cuentasBancarias,
        summary: getSummary(calculated.totalCents, paid.cents),
        diagnostics,
      })
    } catch {
      return storageFailure()
    }
  },

  async save(command) {
    const normalized = normalizeCuenta(command.cuenta)
    if (!normalized.ok) return normalized

    try {
      return saveTransaction.immediate(command, normalized.value)
    } catch {
      return storageFailure()
    }
  },
}
