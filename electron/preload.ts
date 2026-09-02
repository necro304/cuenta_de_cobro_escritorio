import { contextBridge, ipcRenderer } from 'electron'
import type { CuentaDeCobroTarget, SaveCuentaDeCobroCommand } from '../src/types/cuentaDeCobro'

type DbParam = string | number | null | boolean

contextBridge.exposeInMainWorld('electronAPI', {
  cuentaDeCobro: {
    open: (target: CuentaDeCobroTarget) => ipcRenderer.invoke('cuenta-de-cobro:open', target),
    save: (command: SaveCuentaDeCobroCommand) =>
      ipcRenderer.invoke('cuenta-de-cobro:save', command),
  },
  dbQuery: (sql: string, params?: DbParam[]) => ipcRenderer.invoke('db-query', sql, params),
  dbGet: (sql: string, params?: DbParam[]) => ipcRenderer.invoke('db-get', sql, params),
  dbRun: (sql: string, params?: DbParam[]) => ipcRenderer.invoke('db-run', sql, params),
  dbBackup: () => ipcRenderer.invoke('db-backup'),
  dbRestore: () => ipcRenderer.invoke('db-restore'),
  checkForUpdates: () => ipcRenderer.invoke('check-for-updates'),
  exportPdf: (opts: {
    invoiceId: number
    invoiceNumber: number
    template: string
    includeSignature: boolean
  }) => ipcRenderer.invoke('export-pdf', opts),
  exportClientReportPdf: (opts: { clientId: number; clientName: string }) =>
    ipcRenderer.invoke('export-client-report-pdf', opts),
  notifyPrintReady: () => ipcRenderer.send('print-ready'),
})
