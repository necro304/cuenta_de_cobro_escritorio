export type DbParam = string | number | null | boolean

export interface ElectronAPI {
  dbQuery: <T = unknown>(sql: string, params?: DbParam[]) => Promise<T[]>
  dbGet: <T = unknown>(sql: string, params?: DbParam[]) => Promise<T | undefined>
  dbRun: (
    sql: string,
    params?: DbParam[],
  ) => Promise<{ changes: number; lastInsertRowid: number | bigint }>
  dbBackup: () => Promise<{ success: boolean; message: string }>
  dbRestore: () => Promise<{ success: boolean; message: string }>
  checkForUpdates: () => Promise<{ updateInfo?: { version: string } } | null>
  exportPdf: (opts: {
    invoiceId: number
    invoiceNumber: number
    template: string
    includeSignature: boolean
  }) => Promise<{ success: boolean; message: string }>
  notifyPrintReady: () => void
}

declare global {
  interface Window {
    electronAPI: ElectronAPI
  }
}
