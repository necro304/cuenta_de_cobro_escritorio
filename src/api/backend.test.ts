import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  BackendError,
  CANCELLED_MESSAGE,
  DESKTOP_ONLY_MESSAGE,
  backupDatabase,
  cuentaDeCobro,
  dbGet,
  dbQuery,
  dbRun,
  exportClientReportPdf,
  exportInvoicePdf,
  installCli,
  legacyMigrationNotice,
  notifyPrintReady,
  restoreDatabase,
} from './backend'

type TauriGlobal = typeof globalThis & { isTauri?: boolean }

interface IpcCall {
  cmd: string
  payload: unknown
}

let calls: IpcCall[] = []

const mockBackend = (respond: (cmd: string, payload: unknown) => unknown) => {
  calls = []
  mockIPC((cmd, payload) => {
    calls.push({ cmd, payload })
    return respond(cmd, payload)
  })
}

describe('backend (contrato IPC)', () => {
  beforeEach(() => {
    // mockIPC no marca el entorno como Tauri; isTauri() lee globalThis.isTauri
    ;(globalThis as TauriGlobal).isTauri = true
  })

  afterEach(() => {
    clearMocks()
    delete (globalThis as TauriGlobal).isTauri
  })

  it('envía SQL y parámetros a db_query con params [] por defecto', async () => {
    mockBackend(() => [{ id: 1 }])

    await expect(dbQuery<{ id: number }>('SELECT id FROM clients')).resolves.toEqual([{ id: 1 }])
    expect(calls).toEqual([
      { cmd: 'db_query', payload: { sql: 'SELECT id FROM clients', params: [] } },
    ])
  })

  it('normaliza la fila ausente de db_get (null) a undefined', async () => {
    mockBackend(() => null)

    await expect(dbGet('SELECT * FROM clients WHERE id = ?', [99])).resolves.toBeUndefined()
    expect(calls[0]).toEqual({
      cmd: 'db_get',
      payload: { sql: 'SELECT * FROM clients WHERE id = ?', params: [99] },
    })
  })

  it('devuelve changes y lastInsertRowid de db_run', async () => {
    mockBackend(() => ({ changes: 1, lastInsertRowid: 42 }))

    await expect(dbRun('DELETE FROM clients WHERE id = ?', [3])).resolves.toEqual({
      changes: 1,
      lastInsertRowid: 42,
    })
  })

  it('convierte el rechazo del backend en BackendError con el comando', async () => {
    mockBackend(() => Promise.reject('FOREIGN KEY constraint failed'))

    const error = await dbRun('DELETE FROM clients WHERE id = ?', [1]).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(BackendError)
    expect(error).toMatchObject({ command: 'db_run', message: 'FOREIGN KEY constraint failed' })
  })

  it('toma el message de un error estructurado del backend', async () => {
    mockBackend(() => Promise.reject({ code: 'SQLITE_ERROR', message: 'no such table: foo' }))

    await expect(dbQuery('SELECT * FROM foo')).rejects.toThrow('no such table: foo')
  })

  it('usa las claves target y command para el módulo de cuenta de cobro', async () => {
    mockBackend(() => ({ ok: false, error: { code: 'STORAGE_FAILURE' } }))

    await cuentaDeCobro.open({ kind: 'edit', id: 5 })
    await cuentaDeCobro.save({
      kind: 'create',
      cuenta: {
        number: '7',
        date: '2026-11-11',
        clientId: 1,
        bankAccountId: 2,
        notes: '',
        concepts: [{ description: 'Servicio', quantity: '1', price: '100' }],
      },
    })

    expect(calls[0]).toEqual({
      cmd: 'cuenta_de_cobro_open',
      payload: { target: { kind: 'edit', id: 5 } },
    })
    expect(calls[1].cmd).toBe('cuenta_de_cobro_save')
    expect(calls[1].payload).toMatchObject({ command: { kind: 'create', cuenta: { number: '7' } } })
  })

  it('envía los PDF con la clave opts y campos camelCase', async () => {
    mockBackend(() => ({ success: true, message: '/tmp/x.pdf' }))

    await expect(
      exportInvoicePdf({
        invoiceId: 3,
        invoiceNumber: 12,
        template: 'simple',
        includeSignature: true,
      }),
    ).resolves.toEqual({ status: 'done', message: '/tmp/x.pdf' })
    await exportClientReportPdf({ clientId: 7, clientName: 'Fundación Horizonte' })

    expect(calls).toEqual([
      {
        cmd: 'export_pdf',
        payload: {
          opts: { invoiceId: 3, invoiceNumber: 12, template: 'simple', includeSignature: true },
        },
      },
      {
        cmd: 'export_client_report_pdf',
        payload: { opts: { clientId: 7, clientName: 'Fundación Horizonte' } },
      },
    ])
  })

  it('interpreta success/message como done, cancelled o failed', async () => {
    mockBackend((cmd) =>
      cmd === 'db_backup'
        ? { success: true, message: 'Respaldo guardado exitosamente.' }
        : { success: false, message: CANCELLED_MESSAGE },
    )

    await expect(backupDatabase()).resolves.toEqual({
      status: 'done',
      message: 'Respaldo guardado exitosamente.',
    })
    await expect(restoreDatabase()).resolves.toEqual({ status: 'cancelled' })
    expect(calls.map((call) => call.payload)).toEqual([{}, {}])

    mockBackend(() => ({ success: false, message: 'Tiempo de espera agotado generando el PDF' }))
    await expect(
      exportInvoicePdf({
        invoiceId: 1,
        invoiceNumber: 1,
        template: 'default',
        includeSignature: false,
      }),
    ).resolves.toEqual({ status: 'failed', message: 'Tiempo de espera agotado generando el PDF' })
  })

  it('las acciones con diálogo nunca rechazan: el error llega como failed', async () => {
    mockBackend(() => Promise.reject('disco lleno'))

    await expect(backupDatabase()).resolves.toEqual({ status: 'failed', message: 'disco lleno' })
  })

  it('installCli invoca install_cli sin argumentos y respeta la cancelación', async () => {
    mockBackend(() => ({ success: false, message: CANCELLED_MESSAGE }))

    await expect(installCli()).resolves.toEqual({ status: 'cancelled' })
    expect(calls).toEqual([{ cmd: 'install_cli', payload: {} }])
  })

  it('notifyPrintReady invoca print_ready sin argumentos y no rechaza', async () => {
    mockBackend(() => Promise.reject('sin exportación pendiente'))

    await expect(notifyPrintReady()).resolves.toBeUndefined()
    expect(calls[0].cmd).toBe('print_ready')
  })

  it('legacyMigrationNotice invoca legacy_migration_notice sin argumentos', async () => {
    const notice = { source: '/legacy/database.sqlite', migratedAtUnix: 1794401583, os: 'macos' }
    mockBackend(() => notice)

    await expect(legacyMigrationNotice()).resolves.toEqual(notice)
    expect(calls).toEqual([{ cmd: 'legacy_migration_notice', payload: {} }])
  })

  it('legacyMigrationNotice devuelve null si no hubo migración o si el comando falla', async () => {
    mockBackend(() => null)
    await expect(legacyMigrationNotice()).resolves.toBeNull()

    mockBackend(() => Promise.reject('fallo de disco'))
    await expect(legacyMigrationNotice()).resolves.toBeNull()
  })
})

describe('backend fuera de Tauri', () => {
  it('rechaza el SQL con un mensaje claro en lugar de un TypeError', async () => {
    await expect(dbQuery('SELECT 1')).rejects.toThrow(DESKTOP_ONLY_MESSAGE)
  })

  it('las acciones devuelven failed y notifyPrintReady es un no-op', async () => {
    await expect(backupDatabase()).resolves.toEqual({
      status: 'failed',
      message: DESKTOP_ONLY_MESSAGE,
    })
    await expect(notifyPrintReady()).resolves.toBeUndefined()
    await expect(legacyMigrationNotice()).resolves.toBeNull()
  })
})
