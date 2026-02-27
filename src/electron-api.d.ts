export type DbParam = string | number | null | boolean

export interface ElectronAPI {
  dbQuery: <T = unknown>(sql: string, params?: DbParam[]) => Promise<T[]>
  dbGet: <T = unknown>(sql: string, params?: DbParam[]) => Promise<T | undefined>
  dbRun: (sql: string, params?: DbParam[]) => Promise<{ changes: number; lastInsertRowid: number | bigint }>
  dbBackup: () => Promise<{ success: boolean; message: string }>
  dbRestore: () => Promise<{ success: boolean; message: string }>
}

declare global {
  interface Window {
    electronAPI: ElectronAPI
  }
}
