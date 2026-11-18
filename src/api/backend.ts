/**
 * Capa tipada sobre el IPC de Tauri. Es el ÚNICO módulo del renderer que importa
 * `@tauri-apps/api/core`; vistas, stores y tests dependen solo de estas funciones.
 *
 * Contrato: los nombres de comando son snake_case (Rust) y las claves de primer
 * nivel de `invoke` van en camelCase (conversión por defecto de Tauri 2). Los
 * diálogos de archivo los abre Rust (paridad con Electron).
 */
import { invoke, isTauri } from '@tauri-apps/api/core'
import type { TemplateId } from '@/types'
import type {
  CuentaDeCobroEditorModel,
  CuentaDeCobroModule,
  CuentaDeCobroResult,
  CuentaDeCobroTarget,
  SaveCuentaDeCobroCommand,
  SaveCuentaDeCobroReceipt,
} from '@/types/cuentaDeCobro'

/**
 * Valor que puede enlazarse a un `?` de SQLite. Igual que better-sqlite3, Rust rechaza
 * booleanos, arrays y objetos: convierte un booleano con `flag ? 1 : 0`.
 */
export type DbParam = string | number | null

/** Resultado de `dbRun` (mismo shape que better-sqlite3). */
export interface RunResult {
  changes: number
  lastInsertRowid: number
}

/**
 * Resultado de una acción con diálogo nativo (respaldo, restauración, PDF, instalar la CLI), ya
 * interpretado para las vistas. `done.message` es texto para el usuario (en PDF:
 * la ruta del archivo guardado).
 */
export type ActionOutcome =
  | { status: 'done'; message: string }
  | { status: 'cancelled' }
  | { status: 'failed'; message: string }

/** Lo que responden en el cable `db_backup`, `db_restore` y los comandos de PDF. */
export interface OperationResult {
  success: boolean
  message: string
}

/** Mensaje con el que Rust señala que el usuario cerró el diálogo (igual que Electron). */
export const CANCELLED_MESSAGE = 'Operación cancelada.'

export interface ExportInvoicePdfRequest {
  invoiceId: number
  invoiceNumber: number
  template: TemplateId
  includeSignature: boolean
}

export interface ExportClientReportPdfRequest {
  clientId: number
  clientName: string
}

export type BackendCommand =
  | 'db_query'
  | 'db_get'
  | 'db_run'
  | 'cuenta_de_cobro_open'
  | 'cuenta_de_cobro_save'
  | 'db_backup'
  | 'db_restore'
  | 'clear_billing_data'
  | 'export_pdf'
  | 'export_client_report_pdf'
  | 'print_ready'
  | 'legacy_migration_notice'
  | 'install_cli'

/** Aviso único tras importar la BD de la versión Electron (`legacy-migration.json`). */
export interface LegacyMigrationNotice {
  /** Ruta absoluta de la BD heredada (el original queda intacto). */
  source: string
  migratedAtUnix: number
  os: 'macos' | 'windows' | 'linux'
}

export const DESKTOP_ONLY_MESSAGE =
  'Esta función solo está disponible en la aplicación de escritorio.'

/** Error de cualquier comando: IPC caído, SQL inválido, restricción violada, etc. */
export class BackendError extends Error {
  readonly command: BackendCommand

  constructor(command: BackendCommand, message: string) {
    super(message)
    this.name = 'BackendError'
    this.command = command
  }
}

/** `true` dentro de la app Tauri; `false` en navegador, `vite preview` y Vitest. */
export const isDesktop = (): boolean => isTauri()

/** Rust rechaza con un String; se toleran también Error y objetos `{ message }`. */
const describeRejection = (reason: unknown): string => {
  if (reason instanceof Error) return reason.message
  if (typeof reason === 'string') return reason
  if (
    typeof reason === 'object' &&
    reason !== null &&
    'message' in reason &&
    typeof reason.message === 'string'
  ) {
    return reason.message
  }
  return 'Error desconocido del backend.'
}

async function call<T>(command: BackendCommand, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop()) throw new BackendError(command, DESKTOP_ONLY_MESSAGE)
  try {
    return await invoke<T>(command, args)
  } catch (reason) {
    throw new BackendError(command, describeRejection(reason))
  }
}

const toOutcome = ({ success, message }: OperationResult): ActionOutcome => {
  if (success) return { status: 'done', message }
  if (message === CANCELLED_MESSAGE) return { status: 'cancelled' }
  return { status: 'failed', message }
}

/** Para acciones con diálogo: nunca rechaza, todo fallo llega como `failed`. */
async function runAction(
  command: BackendCommand,
  args?: Record<string, unknown>,
): Promise<ActionOutcome> {
  try {
    return toOutcome(await call<OperationResult>(command, args))
  } catch (error) {
    return { status: 'failed', message: describeRejection(error) }
  }
}

// --- SQL genérico -----------------------------------------------------------

export const dbQuery = <T = unknown>(sql: string, params: DbParam[] = []): Promise<T[]> =>
  call<T[]>('db_query', { sql, params })

export const dbGet = async <T = unknown>(
  sql: string,
  params: DbParam[] = [],
): Promise<T | undefined> => (await call<T | null>('db_get', { sql, params })) ?? undefined

export const dbRun = (sql: string, params: DbParam[] = []): Promise<RunResult> =>
  call<RunResult>('db_run', { sql, params })

// --- Cuenta de cobro (módulo profundo en Rust) ------------------------------

export const cuentaDeCobro: CuentaDeCobroModule = {
  open: (target: CuentaDeCobroTarget) =>
    call<CuentaDeCobroResult<CuentaDeCobroEditorModel>>('cuenta_de_cobro_open', { target }),
  save: (command: SaveCuentaDeCobroCommand) =>
    call<CuentaDeCobroResult<SaveCuentaDeCobroReceipt>>('cuenta_de_cobro_save', { command }),
}

// --- Respaldo / restauración -------------------------------------------------

export const backupDatabase = (): Promise<ActionOutcome> => runAction('db_backup')

export const clearBillingData = (): Promise<void> => call<void>('clear_billing_data')

/**
 * En éxito Rust reinicia la app (`app.restart()`), así que la promesa no llega a
 * resolver. `done` queda solo como respaldo: si algún día Rust respondiera éxito
 * sin reiniciar, quien llama invoca `relaunch()`.
 */
export const restoreDatabase = (): Promise<ActionOutcome> => runAction('db_restore')

// --- PDF ---------------------------------------------------------------------

export const exportInvoicePdf = (request: ExportInvoicePdfRequest): Promise<ActionOutcome> =>
  runAction('export_pdf', { opts: { ...request } })

export const exportClientReportPdf = (
  request: ExportClientReportPdfRequest,
): Promise<ActionOutcome> => runAction('export_client_report_pdf', { opts: { ...request } })

/**
 * Lo llama la vista de impresión cuando corre en la ventana oculta de exportación
 * (`?pdf=1`). Fuera de Tauri no hace nada. Nunca rechaza: si la señal se pierde,
 * el backend agota su tiempo de espera y la exportación termina en `failed`.
 */
export const notifyPrintReady = async (): Promise<void> => {
  if (!isDesktop()) return
  try {
    await call<null>('print_ready')
  } catch {
    // El timeout del comando de exportación informa el fallo al usuario.
  }
}

// --- CLI para agentes ----------------------------------------------------------

/** Pone `cuentacobro-cli` en el PATH. En macOS pide la contraseña: cerrarla da `cancelled`. */
export const installCli = (): Promise<ActionOutcome> => runAction('install_cli')

// --- Migración desde Electron -----------------------------------------------

/**
 * Devuelve el aviso de importación UNA sola vez (Rust lo marca como mostrado).
 * Nunca rechaza: fuera de Tauri o ante cualquier error devuelve `null`.
 */
export const legacyMigrationNotice = async (): Promise<LegacyMigrationNotice | null> => {
  if (!isDesktop()) return null
  try {
    return await call<LegacyMigrationNotice | null>('legacy_migration_notice')
  } catch {
    return null
  }
}
