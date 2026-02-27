import { contextBridge, ipcRenderer } from 'electron'

type DbParam = string | number | null | boolean

contextBridge.exposeInMainWorld('electronAPI', {
  dbQuery: (sql: string, params?: DbParam[]) => ipcRenderer.invoke('db-query', sql, params),
  dbGet: (sql: string, params?: DbParam[]) => ipcRenderer.invoke('db-get', sql, params),
  dbRun: (sql: string, params?: DbParam[]) => ipcRenderer.invoke('db-run', sql, params),
  dbBackup: () => ipcRenderer.invoke('db-backup'),
  dbRestore: () => ipcRenderer.invoke('db-restore'),
})
