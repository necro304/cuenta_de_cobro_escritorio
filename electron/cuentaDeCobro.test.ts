import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { app } from 'electron'

const testDirectory = mkdtempSync(path.join(tmpdir(), 'cuenta-de-cobro-test-'))
app.setPath('userData', testDirectory)

const run = async () => {
  let closeDatabase: (() => void) | undefined
  let exitCode = 0

  try {
    const { default: db, initDb } = await import('./database')
    closeDatabase = () => db.close()
    initDb()

    const client = db.prepare('INSERT INTO clients (name) VALUES (?)').run('Cliente de prueba')
    const bankAccount = db
      .prepare(
        'INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, 1)',
      )
      .run('Banco de prueba', 'Ahorros', '123456')

    const { cuentaDeCobroModule } = await import('./cuentaDeCobro')
    const opened = await cuentaDeCobroModule.open({ kind: 'create' })

    assert.equal(opened.ok, true)
    if (!opened.ok) throw new Error('No se pudo abrir el editor de creación')

    assert.equal(opened.value.cuenta.number, '1')
    assert.equal(opened.value.cuenta.bankAccountId, Number(bankAccount.lastInsertRowid))

    const saved = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        clientId: Number(client.lastInsertRowid),
        concepts: [
          { description: 'Concepto A', quantity: '1', price: '0.335' },
          { description: 'Concepto B', quantity: '1', price: '0.335' },
        ],
      },
    })

    assert.deepEqual(saved, {
      ok: true,
      value: { id: 1, number: 1, total: '0.68', status: 'draft' },
    })

    const reloaded = await cuentaDeCobroModule.open({ kind: 'edit', id: 1 })
    assert.equal(reloaded.ok, true)
    if (!reloaded.ok) throw new Error('No se pudo volver a abrir la cuenta de cobro')

    assert.equal(reloaded.value.cuenta.concepts.length, 2)
    assert.equal(reloaded.value.summary.total, '0.68')
    assert.equal(reloaded.value.summary.paymentStatus, 'draft')

    console.warn('✓ crea y abre una cuenta de cobro con redondeo por concepto')

    const invalid = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '2',
        clientId: Number(client.lastInsertRowid),
        concepts: [{ description: 'Sin valor', quantity: '1', price: '0' }],
      },
    })
    assert.equal(invalid.ok, false)
    if (invalid.ok) throw new Error('La cuenta inválida fue aceptada')
    assert.equal(invalid.error.code, 'VALIDATION_FAILED')

    const duplicate = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '1',
        clientId: Number(client.lastInsertRowid),
        concepts: [{ description: 'Duplicado', quantity: '1', price: '1' }],
      },
    })
    assert.equal(duplicate.ok, false)
    if (duplicate.ok) throw new Error('El número duplicado fue aceptado')
    assert.deepEqual(duplicate.error, {
      code: 'NUMBER_IN_USE',
      number: 1,
      suggestedNumber: 2,
    })

    const missingClient = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '2',
        clientId: 999,
        concepts: [{ description: 'Referencia', quantity: '1', price: '1' }],
      },
    })
    assert.equal(missingClient.ok, false)
    if (missingClient.ok) throw new Error('El cliente inexistente fue aceptado')
    assert.deepEqual(missingClient.error, { code: 'CLIENTE_NOT_FOUND', id: 999 })

    const missingBankAccount = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '2',
        clientId: Number(client.lastInsertRowid),
        bankAccountId: 999,
        concepts: [{ description: 'Referencia', quantity: '1', price: '1' }],
      },
    })
    assert.equal(missingBankAccount.ok, false)
    if (missingBankAccount.ok) throw new Error('La cuenta bancaria inexistente fue aceptada')
    assert.deepEqual(missingBankAccount.error, { code: 'CUENTA_BANCARIA_NOT_FOUND', id: 999 })

    console.warn('✓ rechaza valores inválidos, números duplicados y referencias inexistentes')

    db.prepare(
      'INSERT INTO invoice_payments (invoice_id, date, amount, notes) VALUES (?, ?, ?, ?)',
    ).run(1, '2026-08-27', 0.5, '')

    const belowPaidAmount = await cuentaDeCobroModule.save({
      kind: 'edit',
      id: 1,
      cuenta: {
        ...reloaded.value.cuenta,
        concepts: [{ description: 'Total reducido', quantity: '1', price: '0.40' }],
      },
    })
    assert.equal(belowPaidAmount.ok, false)
    if (belowPaidAmount.ok) throw new Error('Se creó un sobrepago mediante edición')
    assert.deepEqual(belowPaidAmount.error, {
      code: 'TOTAL_BELOW_PAID_AMOUNT',
      total: '0.40',
      paidAmount: '0.50',
    })

    const afterRejectedEdit = await cuentaDeCobroModule.open({ kind: 'edit', id: 1 })
    assert.equal(afterRejectedEdit.ok, true)
    if (!afterRejectedEdit.ok) throw new Error('No se pudo verificar la edición rechazada')
    assert.equal(afterRejectedEdit.value.summary.total, '0.68')
    assert.equal(afterRejectedEdit.value.cuenta.concepts.length, 2)

    const paid = await cuentaDeCobroModule.save({
      kind: 'edit',
      id: 1,
      cuenta: {
        ...reloaded.value.cuenta,
        notes: '  Cuenta pagada  ',
        concepts: [{ description: '  Total pagado  ', quantity: '1', price: '0.50' }],
      },
    })
    assert.deepEqual(paid, {
      ok: true,
      value: { id: 1, number: 1, total: '0.50', status: 'paid' },
    })

    const paidReloaded = await cuentaDeCobroModule.open({ kind: 'edit', id: 1 })
    assert.equal(paidReloaded.ok, true)
    if (!paidReloaded.ok) throw new Error('No se pudo verificar la cuenta pagada')
    assert.equal(paidReloaded.value.cuenta.notes, 'Cuenta pagada')
    assert.equal(paidReloaded.value.cuenta.concepts[0].description, 'Total pagado')
    assert.equal(paidReloaded.value.summary.paymentStatus, 'paid')

    console.warn('✓ protege los abonos, normaliza texto y deriva el estado de pago')

    const duplicateInvoice = db
      .prepare(
        'INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, ?, ?, ?, ?, ?, ?)',
      )
      .run(
        1,
        '2026-08-27',
        Number(client.lastInsertRowid),
        Number(bankAccount.lastInsertRowid),
        1,
        '',
        'draft',
      )
    db.prepare(
      'INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)',
    ).run(Number(duplicateInvoice.lastInsertRowid), 'Histórico', 1, 1)

    const historicDuplicate = await cuentaDeCobroModule.open({ kind: 'edit', id: 1 })
    assert.equal(historicDuplicate.ok, true)
    if (!historicDuplicate.ok) throw new Error('No se pudo abrir el duplicado histórico')
    assert.deepEqual(historicDuplicate.value.diagnostics, [
      { code: 'HISTORIC_NUMBER_CONFLICT', number: 1 },
    ])

    db.pragma('foreign_keys = OFF')
    const invalidHistoricInvoice = db
      .prepare(
        'INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, ?, ?, ?, ?, ?, ?)',
      )
      .run(4, '2026-08-27', 998, 999, 0, '', 'draft')
    db.prepare(
      'INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)',
    ).run(Number(invalidHistoricInvoice.lastInsertRowid), 'Sin valor', 1, 0)
    db.pragma('foreign_keys = ON')

    const invalidHistoric = await cuentaDeCobroModule.open({
      kind: 'edit',
      id: Number(invalidHistoricInvoice.lastInsertRowid),
    })
    assert.equal(invalidHistoric.ok, true)
    if (!invalidHistoric.ok) throw new Error('No se pudo abrir la cuenta histórica inválida')
    assert.deepEqual(invalidHistoric.value.diagnostics, [
      { code: 'MISSING_CLIENT_REFERENCE', clientId: 998 },
      { code: 'MISSING_BANK_ACCOUNT_REFERENCE', bankAccountId: 999 },
      { code: 'INVALID_PERSISTED_CONCEPTS' },
    ])
    assert.equal(invalidHistoric.value.cuenta.clientId, null)
    assert.equal(invalidHistoric.value.cuenta.bankAccountId, null)

    console.warn('✓ abre datos históricos inválidos sin corregirlos silenciosamente')

    db.exec(`
      CREATE TRIGGER fail_invoice_item_insert
      BEFORE INSERT ON invoice_items
      WHEN NEW.description = 'Forzar rollback'
      BEGIN
        SELECT RAISE(ABORT, 'forced test failure');
      END;
    `)

    const rolledBack = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '5',
        clientId: Number(client.lastInsertRowid),
        concepts: [
          { description: 'Primero', quantity: '1', price: '1' },
          { description: 'Forzar rollback', quantity: '1', price: '1' },
        ],
      },
    })
    assert.deepEqual(rolledBack, { ok: false, error: { code: 'STORAGE_FAILURE' } })
    db.exec('DROP TRIGGER fail_invoice_item_insert')

    const retryAfterRollback = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '5',
        clientId: Number(client.lastInsertRowid),
        concepts: [{ description: 'Después del rollback', quantity: '1', price: '1' }],
      },
    })
    assert.equal(retryAfterRollback.ok, true)

    console.warn('✓ revierte cabecera y conceptos cuando falla una escritura intermedia')

    if (!retryAfterRollback.ok) throw new Error('No existe la cuenta para probar el rollback')
    const beforeRolledBackEdit = await cuentaDeCobroModule.open({
      kind: 'edit',
      id: retryAfterRollback.value.id,
    })
    assert.equal(beforeRolledBackEdit.ok, true)
    if (!beforeRolledBackEdit.ok) throw new Error('No se abrió la cuenta para editar')

    db.exec(`
      CREATE TRIGGER fail_invoice_item_edit
      BEFORE INSERT ON invoice_items
      WHEN NEW.description = 'Forzar rollback de edición'
      BEGIN
        SELECT RAISE(ABORT, 'forced edit failure');
      END;
    `)

    const rolledBackEdit = await cuentaDeCobroModule.save({
      kind: 'edit',
      id: retryAfterRollback.value.id,
      cuenta: {
        ...beforeRolledBackEdit.value.cuenta,
        number: '8',
        date: '2030-01-01',
        notes: 'No debe persistir',
        concepts: [{ description: 'Forzar rollback de edición', quantity: '1', price: '0.50' }],
      },
    })
    assert.deepEqual(rolledBackEdit, { ok: false, error: { code: 'STORAGE_FAILURE' } })
    db.exec('DROP TRIGGER fail_invoice_item_edit')

    const afterRolledBackEdit = await cuentaDeCobroModule.open({
      kind: 'edit',
      id: retryAfterRollback.value.id,
    })
    assert.equal(afterRolledBackEdit.ok, true)
    if (!afterRolledBackEdit.ok) throw new Error('No se pudo verificar el rollback de edición')
    assert.equal(afterRolledBackEdit.value.cuenta.number, beforeRolledBackEdit.value.cuenta.number)
    assert.equal(afterRolledBackEdit.value.cuenta.date, beforeRolledBackEdit.value.cuenta.date)
    assert.equal(afterRolledBackEdit.value.cuenta.notes, beforeRolledBackEdit.value.cuenta.notes)
    assert.equal(afterRolledBackEdit.value.summary.total, beforeRolledBackEdit.value.summary.total)
    assert.equal(
      afterRolledBackEdit.value.summary.paymentStatus,
      beforeRolledBackEdit.value.summary.paymentStatus,
    )
    assert.equal(afterRolledBackEdit.value.cuenta.concepts[0].description, 'Después del rollback')

    console.warn('✓ restaura los conceptos anteriores cuando falla una edición')

    const scientificDecimal = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '6',
        clientId: Number(client.lastInsertRowid),
        concepts: [{ description: 'Decimal pequeño', quantity: '100000', price: '0.0000001' }],
      },
    })
    assert.equal(scientificDecimal.ok, true)
    if (!scientificDecimal.ok) throw new Error('No se guardó el decimal pequeño')
    assert.equal(scientificDecimal.value.total, '0.01')

    const scientificReloaded = await cuentaDeCobroModule.open({
      kind: 'edit',
      id: scientificDecimal.value.id,
    })
    assert.equal(scientificReloaded.ok, true)
    if (!scientificReloaded.ok) throw new Error('No se reabrió el decimal pequeño')
    assert.equal(scientificReloaded.value.cuenta.concepts[0].price, '0.0000001')

    const scientificResaved = await cuentaDeCobroModule.save({
      kind: 'edit',
      id: scientificDecimal.value.id,
      cuenta: scientificReloaded.value.cuenta,
    })
    assert.equal(scientificResaved.ok, true)

    const impreciseAmount = await cuentaDeCobroModule.save({
      kind: 'create',
      cuenta: {
        ...opened.value.cuenta,
        number: '7',
        clientId: Number(client.lastInsertRowid),
        concepts: [{ description: 'No representable', quantity: '1', price: '90071992547409.91' }],
      },
    })
    assert.equal(impreciseAmount.ok, false)
    if (impreciseAmount.ok) throw new Error('Se aceptó un decimal que SQLite altera')
    assert.equal(impreciseAmount.error.code, 'VALIDATION_FAILED')

    console.warn('✓ conserva decimales pequeños y rechaza valores que SQLite alteraría')

    db.pragma('foreign_keys = OFF')
    const malformedInvoice = db
      .prepare(
        'INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, ?, ?, ?, ?, ?, ?)',
      )
      .run(0, '2026-02-30', Number(client.lastInsertRowid), null, null, '', 'draft')
    db.prepare(
      'INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)',
    ).run(Number(malformedInvoice.lastInsertRowid), 'Reparable', 1, 1)
    db.pragma('foreign_keys = ON')

    const malformedOpened = await cuentaDeCobroModule.open({
      kind: 'edit',
      id: Number(malformedInvoice.lastInsertRowid),
    })
    assert.equal(malformedOpened.ok, true)
    if (!malformedOpened.ok) throw new Error('No se abrió la cuenta histórica dañada')
    assert.deepEqual(malformedOpened.value.diagnostics, [
      { code: 'INVALID_PERSISTED_ACCOUNT' },
      { code: 'MISSING_BANK_ACCOUNT_REFERENCE', bankAccountId: null },
    ])

    const inconsistentTotalInvoice = db
      .prepare(
        'INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes, status) VALUES (?, ?, ?, ?, ?, ?, ?)',
      )
      .run(
        8,
        '2026-08-27',
        Number(client.lastInsertRowid),
        Number(bankAccount.lastInsertRowid),
        999,
        '',
        'draft',
      )
    db.prepare(
      'INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)',
    ).run(Number(inconsistentTotalInvoice.lastInsertRowid), 'Total real', 1, 1)

    const inconsistentTotalOpened = await cuentaDeCobroModule.open({
      kind: 'edit',
      id: Number(inconsistentTotalInvoice.lastInsertRowid),
    })
    assert.equal(inconsistentTotalOpened.ok, true)
    if (!inconsistentTotalOpened.ok) throw new Error('No se abrió el total inconsistente')
    assert.deepEqual(inconsistentTotalOpened.value.diagnostics, [
      { code: 'INVALID_PERSISTED_ACCOUNT' },
    ])
    assert.equal(inconsistentTotalOpened.value.summary.total, '1.00')

    db.prepare(
      'INSERT INTO invoice_payments (invoice_id, date, amount, notes) VALUES (?, ?, ?, ?)',
    ).run(retryAfterRollback.value.id, '2026-08-27', -1, '')
    const invalidPaymentOpened = await cuentaDeCobroModule.open({
      kind: 'edit',
      id: retryAfterRollback.value.id,
    })
    assert.equal(invalidPaymentOpened.ok, true)
    if (!invalidPaymentOpened.ok) throw new Error('No se abrió la cuenta con abono inválido')
    assert.deepEqual(invalidPaymentOpened.value.diagnostics, [
      { code: 'INVALID_PERSISTED_PAYMENTS' },
    ])

    console.warn('✓ diagnostica cabeceras y abonos históricos inválidos sin bloquear open')
  } catch (error) {
    exitCode = 1
    console.error(error)
  } finally {
    closeDatabase?.()
    rmSync(testDirectory, { recursive: true, force: true })
    app.exit(exitCode)
  }
}

void app.whenReady().then(run)
