import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import fs from 'node:fs'
import db, { initDb } from './database'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

process.env.APP_ROOT = path.join(__dirname, '..')

export const VITE_DEV_SERVER_URL = process.env['VITE_DEV_SERVER_URL']
export const MAIN_DIST = path.join(process.env.APP_ROOT, 'dist-electron')
export const RENDERER_DIST = path.join(process.env.APP_ROOT, 'dist')

process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL ? path.join(process.env.APP_ROOT, 'public') : RENDERER_DIST

let win: BrowserWindow | null

function createWindow() {
  win = new BrowserWindow({
    icon: path.join(process.env.VITE_PUBLIC, 'electron-vite.svg'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.mjs'),
    },
  })

  if (VITE_DEV_SERVER_URL) {
    win.loadURL(VITE_DEV_SERVER_URL)
  } else {
    win.loadFile(path.join(RENDERER_DIST, 'index.html'))
  }
}

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
    win = null
  }
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow()
  }
})

app.whenReady().then(() => {
  initDb()

  ipcMain.handle('db-query', (_event, sql: string, params?: unknown[]) => {
    return db.prepare(sql).all(params ?? [])
  })

  ipcMain.handle('db-get', (_event, sql: string, params?: unknown[]) => {
    return db.prepare(sql).get(params ?? [])
  })

  ipcMain.handle('db-run', (_event, sql: string, params?: unknown[]) => {
    return db.prepare(sql).run(params ?? [])
  })

  ipcMain.handle('db-backup', async () => {
    const dbPath = path.join(app.getPath('userData'), 'database.sqlite')
    if (!win) return { success: false, message: 'No window found' }
    
    const { canceled, filePath } = await dialog.showSaveDialog(win, {
      title: 'Respaldar Base de Datos',
      defaultPath: path.join(app.getPath('downloads'), 'respaldo_cuentas_cobro.sqlite'),
      filters: [{ name: 'SQLite Database', extensions: ['sqlite', 'db'] }]
    })

    if (!canceled && filePath) {
      try {
        fs.copyFileSync(dbPath, filePath)
        return { success: true, message: 'Respaldo guardado exitosamente.' }
      } catch (err: any) {
        return { success: false, message: err.message }
      }
    }
    return { success: false, message: 'Operación cancelada.' }
  })

  ipcMain.handle('db-restore', async () => {
    const dbPath = path.join(app.getPath('userData'), 'database.sqlite')
    if (!win) return { success: false, message: 'No window found' }
    
    const { canceled, filePaths } = await dialog.showOpenDialog(win, {
      title: 'Restaurar Base de Datos',
      properties: ['openFile'],
      filters: [{ name: 'SQLite Database', extensions: ['sqlite', 'db'] }]
    })

    if (!canceled && filePaths.length > 0) {
      try {
        // Close DB before overwriting
        db.close()
        // Overwrite the file
        fs.copyFileSync(filePaths[0], dbPath)
        
        // Restart the app to apply the newly loaded database
        app.relaunch()
        app.exit(0)
        
        return { success: true, message: 'Restauración completa. Reiniciando...' }
      } catch (err: any) {
        return { success: false, message: err.message }
      }
    }
    return { success: false, message: 'Operación cancelada.' }
  })

  createWindow()
})
