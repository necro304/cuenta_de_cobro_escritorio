import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { DESKTOP_ONLY_MESSAGE, isDesktop } from '@/api/backend'
import { getErrorMessage } from '@/lib/utils'

export type UpdaterPhase = 'idle' | 'checking' | 'available' | 'downloading' | 'installed' | 'error'

export type ManualCheckResult =
  | { kind: 'available'; version: string }
  | { kind: 'up-to-date' }
  | { kind: 'error'; message: string }

/** En Windows el instalador cierra la app al instalar (plugin-updater). */
export const isWindowsHost = (): boolean =>
  typeof navigator !== 'undefined' && navigator.userAgent.includes('Windows')

/**
 * Estado global de actualización (tauri-plugin-updater + tauri-plugin-process).
 * Una sola instancia para: chequeo al arrancar, botón de Configuración y UpdateDialog.
 */
export const useUpdaterStore = defineStore('updater', () => {
  const phase = ref<UpdaterPhase>('idle')
  const currentVersion = ref<string | null>(null)
  const availableVersion = ref<string | null>(null)
  const releaseNotes = ref<string | null>(null)
  const downloadedBytes = ref(0)
  const totalBytes = ref<number | null>(null)
  const errorMessage = ref<string | null>(null)
  const dialogOpen = ref(false)

  // El Update es un Resource de Rust (rid): fuera del estado reactivo.
  let pendingUpdate: Update | null = null
  let startupCheckDone = false

  const isBusy = computed(() => phase.value === 'checking' || phase.value === 'downloading')
  /** 0-100, o null si el servidor no envía Content-Length. */
  const progressPercent = computed(() =>
    totalBytes.value
      ? Math.min(100, Math.round((downloadedBytes.value / totalBytes.value) * 100))
      : null,
  )

  async function releasePending(): Promise<void> {
    const update = pendingUpdate
    pendingUpdate = null
    if (update) await update.close().catch(() => undefined)
  }

  /** Consulta el endpoint; deja el estado en 'available' o 'idle'. Lanza si falla. */
  async function findUpdate(): Promise<Update | null> {
    phase.value = 'checking'
    errorMessage.value = null
    try {
      await releasePending()
      const update = await check()
      if (!update) {
        phase.value = 'idle'
        return null
      }
      pendingUpdate = update
      currentVersion.value = update.currentVersion
      availableVersion.value = update.version
      releaseNotes.value = update.body?.trim() || null
      phase.value = 'available'
      return update
    } catch (error) {
      phase.value = 'idle'
      throw error
    }
  }

  /**
   * Chequeo silencioso al abrir la ventana principal (paridad con
   * autoUpdater.checkForUpdatesAndNotify). Solo en builds de producción dentro
   * de Tauri; los errores se ignoran.
   */
  async function checkOnStartup(): Promise<void> {
    if (startupCheckDone || !isDesktop() || !import.meta.env.PROD) return
    startupCheckDone = true
    try {
      if (await findUpdate()) dialogOpen.value = true
    } catch {
      // Silencioso: sin red o endpoint caído no debe molestar al abrir la app.
    }
  }

  /** Botón "Buscar actualizaciones". Abre el diálogo si hay versión nueva. */
  async function checkManually(): Promise<ManualCheckResult> {
    if (!isDesktop()) return { kind: 'error', message: DESKTOP_ONLY_MESSAGE }
    if (phase.value === 'installed') {
      dialogOpen.value = true
      return { kind: 'available', version: availableVersion.value ?? '' }
    }
    try {
      const update = await findUpdate()
      if (!update) return { kind: 'up-to-date' }
      dialogOpen.value = true
      return { kind: 'available', version: update.version }
    } catch (error) {
      return { kind: 'error', message: getErrorMessage(error) }
    }
  }

  function onDownloadEvent(event: DownloadEvent): void {
    if (event.event === 'Started') {
      downloadedBytes.value = 0
      totalBytes.value = event.data.contentLength ?? null
    } else if (event.event === 'Progress') {
      downloadedBytes.value += event.data.chunkLength
    }
  }

  /**
   * Descarga e instala. En Windows el proceso termina aquí (el instalador NSIS
   * reabre la app). En macOS/Linux queda en 'installed' y hace falta reiniciar.
   */
  async function install(): Promise<void> {
    const update = pendingUpdate
    if (!update || phase.value === 'downloading') return
    phase.value = 'downloading'
    downloadedBytes.value = 0
    totalBytes.value = null
    errorMessage.value = null
    try {
      await update.downloadAndInstall(onDownloadEvent)
      pendingUpdate = null
      phase.value = 'installed'
    } catch (error) {
      phase.value = 'error'
      errorMessage.value = getErrorMessage(error)
    }
  }

  async function restart(): Promise<void> {
    await relaunch()
  }

  /** "Más tarde"/cerrar. No permitido durante la descarga. */
  function dismiss(): void {
    if (phase.value === 'downloading' || phase.value === 'checking') return
    dialogOpen.value = false
    if (phase.value === 'available' || phase.value === 'error') {
      phase.value = 'idle'
      void releasePending()
    }
  }

  return {
    phase,
    currentVersion,
    availableVersion,
    releaseNotes,
    downloadedBytes,
    totalBytes,
    errorMessage,
    dialogOpen,
    isBusy,
    progressPercent,
    checkOnStartup,
    checkManually,
    install,
    restart,
    dismiss,
  }
})
