import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DownloadEvent } from '@tauri-apps/plugin-updater'
import { useUpdaterStore } from './updater'

const plugins = vi.hoisted(() => ({
  check: vi.fn(),
  relaunch: vi.fn(),
  isDesktop: vi.fn(() => true),
}))

vi.mock('@tauri-apps/plugin-updater', () => ({ check: plugins.check }))
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: plugins.relaunch }))
vi.mock('@/api/backend', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/backend')>()),
  isDesktop: plugins.isDesktop,
}))

const fakeUpdate = (
  downloadAndInstall: (onEvent: (event: DownloadEvent) => void) => Promise<void>,
) => ({
  currentVersion: '1.0.11',
  version: '1.1.0',
  body: 'Mejoras de rendimiento',
  downloadAndInstall: vi.fn(downloadAndInstall),
  close: vi.fn().mockResolvedValue(undefined),
})

describe('useUpdaterStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    plugins.check.mockReset()
    plugins.relaunch.mockReset()
    plugins.isDesktop.mockReset().mockReturnValue(true)
  })

  it('informa que está al día cuando check() devuelve null', async () => {
    plugins.check.mockResolvedValue(null)
    const updater = useUpdaterStore()

    await expect(updater.checkManually()).resolves.toEqual({ kind: 'up-to-date' })
    expect(updater.phase).toBe('idle')
    expect(updater.dialogOpen).toBe(false)
  })

  it('abre el diálogo con versión y notas cuando hay actualización', async () => {
    plugins.check.mockResolvedValue(fakeUpdate(async () => undefined))
    const updater = useUpdaterStore()

    await expect(updater.checkManually()).resolves.toEqual({ kind: 'available', version: '1.1.0' })
    expect(updater.phase).toBe('available')
    expect(updater.dialogOpen).toBe(true)
    expect(updater.releaseNotes).toBe('Mejoras de rendimiento')
  })

  it('reporta el progreso de descarga y queda instalada', async () => {
    let seenPercent: number | null = null
    const update = fakeUpdate(async (onEvent) => {
      onEvent({ event: 'Started', data: { contentLength: 200 } })
      onEvent({ event: 'Progress', data: { chunkLength: 50 } })
      seenPercent = useUpdaterStore().progressPercent
      onEvent({ event: 'Progress', data: { chunkLength: 150 } })
      onEvent({ event: 'Finished' })
    })
    plugins.check.mockResolvedValue(update)
    const updater = useUpdaterStore()
    await updater.checkManually()

    await updater.install()

    expect(seenPercent).toBe(25)
    expect(updater.progressPercent).toBe(100)
    expect(updater.phase).toBe('installed')
    await updater.restart()
    expect(plugins.relaunch).toHaveBeenCalledOnce()
  })

  it('pasa a error con el mensaje si la instalación falla', async () => {
    plugins.check.mockResolvedValue(
      fakeUpdate(async () => {
        throw new Error('firma inválida')
      }),
    )
    const updater = useUpdaterStore()
    await updater.checkManually()

    await updater.install()

    expect(updater.phase).toBe('error')
    expect(updater.errorMessage).toBe('firma inválida')
  })

  it('devuelve error sin llamar a check() fuera de Tauri', async () => {
    plugins.isDesktop.mockReturnValue(false)
    const updater = useUpdaterStore()

    const result = await updater.checkManually()

    expect(result.kind).toBe('error')
    expect(plugins.check).not.toHaveBeenCalled()
  })
})
