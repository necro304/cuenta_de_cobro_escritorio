import { app, BrowserWindow, ipcMain, dialog } from 'electron'
// electron-updater es CJS: sin named exports al importarlo desde ESM
import electronUpdater from 'electron-updater'

const { autoUpdater } = electronUpdater
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import fs from 'node:fs'
import db, { initDb } from './database'
import { cuentaDeCobroModule } from './cuentaDeCobro'
import type { CuentaDeCobroTarget, SaveCuentaDeCobroCommand } from '../src/types/cuentaDeCobro'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

process.env.APP_ROOT = path.join(__dirname, '..')

export const VITE_DEV_SERVER_URL = process.env['VITE_DEV_SERVER_URL']
export const MAIN_DIST = path.join(process.env.APP_ROOT, 'dist-electron')
export const RENDERER_DIST = path.join(process.env.APP_ROOT, 'dist')

process.env.VITE_PUBLIC = VITE_DEV_SERVER_URL
  ? path.join(process.env.APP_ROOT, 'public')
  : RENDERER_DIST

let win: BrowserWindow | null

function createWindow() {
  win = new BrowserWindow({
    width: 1300,
    height: 800,
    minWidth: 640,
    minHeight: 600,
    icon: path.join(process.env.VITE_PUBLIC, 'icon.png'),
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.mjs'),
    },
  })

  if (process.platform === 'darwin') {
    app.dock?.setIcon(path.join(process.env.VITE_PUBLIC, 'icon.png'))
  }

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

  ipcMain.handle('cuenta-de-cobro:open', (_event, target: CuentaDeCobroTarget) => {
    return cuentaDeCobroModule.open(target)
  })

  ipcMain.handle('cuenta-de-cobro:save', (_event, command: SaveCuentaDeCobroCommand) => {
    return cuentaDeCobroModule.save(command)
  })

  ipcMain.handle('db-backup', async () => {
    const dbPath = path.join(app.getPath('userData'), 'database.sqlite')
    if (!win) return { success: false, message: 'No window found' }

    const { canceled, filePath } = await dialog.showSaveDialog(win, {
      title: 'Respaldar Base de Datos',
      defaultPath: path.join(app.getPath('downloads'), 'respaldo_cuentas_cobro.sqlite'),
      filters: [{ name: 'SQLite Database', extensions: ['sqlite', 'db'] }],
    })

    if (!canceled && filePath) {
      try {
        fs.copyFileSync(dbPath, filePath)
        return { success: true, message: 'Respaldo guardado exitosamente.' }
      } catch (err: unknown) {
        return { success: false, message: (err as Error).message }
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
      filters: [{ name: 'SQLite Database', extensions: ['sqlite', 'db'] }],
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
      } catch (err: unknown) {
        return { success: false, message: (err as Error).message }
      }
    }
    return { success: false, message: 'Operación cancelada.' }
  })

  ipcMain.handle('check-for-updates', async () => {
    const result = await autoUpdater.checkForUpdates()
    return result
  })

  // Resolvers de ventanas ocultas esperando la señal 'print-ready' del renderer
  const printReadyResolvers = new Map<number, () => void>()

  ipcMain.on('print-ready', (event) => {
    printReadyResolvers.get(event.sender.id)?.()
  })

  ipcMain.handle(
    'export-pdf',
    async (
      _event,
      opts: {
        invoiceId: number
        invoiceNumber: number
        template: string
        includeSignature: boolean
      },
    ) => {
      if (!win) return { success: false, message: 'No window found' }

      const defaultName = `cuenta_de_cobro_${opts.invoiceNumber.toString().padStart(5, '0')}.pdf`
      const { canceled, filePath } = await dialog.showSaveDialog(win, {
        title: 'Descargar PDF',
        defaultPath: path.join(app.getPath('downloads'), defaultName),
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      })
      if (canceled || !filePath) return { success: false, message: 'Operación cancelada.' }

      // Ventana oculta que renderiza la vista de impresión con plantilla y firma fijadas
      const pdfWin = new BrowserWindow({
        show: false,
        width: 900,
        height: 1200,
        webPreferences: {
          preload: path.join(__dirname, 'preload.mjs'),
        },
      })

      try {
        const hash = `/print/${opts.invoiceId}?pdf=1&template=${opts.template}&signature=${opts.includeSignature ? 1 : 0}`

        const ready = new Promise<void>((resolve, reject) => {
          printReadyResolvers.set(pdfWin.webContents.id, resolve)
          setTimeout(() => reject(new Error('Tiempo de espera agotado generando el PDF')), 10_000)
        })

        if (VITE_DEV_SERVER_URL) {
          await pdfWin.loadURL(`${VITE_DEV_SERVER_URL}#${hash}`)
        } else {
          await pdfWin.loadFile(path.join(RENDERER_DIST, 'index.html'), { hash })
        }

        await ready

        // Mismos márgenes que la impresión (@page { margin: 1.5cm } → pulgadas)
        const margin = 1.5 / 2.54
        const pdfData = await pdfWin.webContents.printToPDF({
          printBackground: true,
          pageSize: 'Letter',
          margins: { top: margin, bottom: margin, left: margin, right: margin },
        })
        fs.writeFileSync(filePath, pdfData)
        return { success: true, message: filePath }
      } catch (err: unknown) {
        return { success: false, message: (err as Error).message }
      } finally {
        printReadyResolvers.delete(pdfWin.webContents.id)
        pdfWin.destroy()
      }
    },
  )

  ipcMain.handle(
    'export-client-report-pdf',
    async (_event, opts: { clientId: number; clientName: string }) => {
      if (!win) return { success: false, message: 'No window found' }

      const safeClientName = opts.clientName
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '_')
        .replace(/^_+|_+$/g, '')
        .slice(0, 60)
      const defaultName = `informe_${safeClientName || `cliente_${opts.clientId}`}.pdf`
      const { canceled, filePath } = await dialog.showSaveDialog(win, {
        title: 'Descargar informe del cliente',
        defaultPath: path.join(app.getPath('downloads'), defaultName),
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      })
      if (canceled || !filePath) return { success: false, message: 'Operación cancelada.' }

      const pdfWin = new BrowserWindow({
        show: false,
        width: 900,
        height: 1200,
        webPreferences: {
          preload: path.join(__dirname, 'preload.mjs'),
        },
      })

      try {
        const hash = `/client-report/${opts.clientId}?pdf=1`
        const ready = new Promise<void>((resolve, reject) => {
          printReadyResolvers.set(pdfWin.webContents.id, resolve)
          setTimeout(
            () => reject(new Error('Tiempo de espera agotado generando el informe')),
            15_000,
          )
        })

        if (VITE_DEV_SERVER_URL) {
          await pdfWin.loadURL(`${VITE_DEV_SERVER_URL}#${hash}`)
        } else {
          await pdfWin.loadFile(path.join(RENDERER_DIST, 'index.html'), { hash })
        }

        await ready
        const pdfData = await pdfWin.webContents.printToPDF({
          printBackground: true,
          pageSize: 'A4',
          preferCSSPageSize: true,
          margins: { top: 0, bottom: 0, left: 0, right: 0 },
        })
        fs.writeFileSync(filePath, pdfData)
        return { success: true, message: filePath }
      } catch (err: unknown) {
        return { success: false, message: (err as Error).message }
      } finally {
        printReadyResolvers.delete(pdfWin.webContents.id)
        pdfWin.destroy()
      }
    },
  )

  createWindow()

  // --- Auto Updater ---
  autoUpdater.checkForUpdatesAndNotify()

  autoUpdater.on('update-available', () => {
    if (win) {
      dialog.showMessageBox(win, {
        type: 'info',
        title: 'Actualización disponible',
        message: 'Una nueva versión está disponible. Se descargará en segundo plano.',
      })
    }
  })

  autoUpdater.on('update-downloaded', () => {
    if (win) {
      dialog
        .showMessageBox(win, {
          type: 'info',
          title: 'Actualización lista',
          message: 'La actualización ha sido descargada. Se instalará al reiniciar la aplicación.',
          buttons: ['Reiniciar ahora', 'Más tarde'],
        })
        .then((result) => {
          if (result.response === 0) {
            autoUpdater.quitAndInstall()
          }
        })
    }
  })

  autoUpdater.on('error', (err) => {
    console.error('Error in auto-updater: ', err)
  })
})
