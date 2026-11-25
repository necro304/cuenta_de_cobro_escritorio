import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { h } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import MainLayout from './MainLayout.vue'
import { useUpdaterStore } from '@/stores/updater'
import type { LegacyMigrationNotice } from '@/api/backend'

const backend = vi.hoisted(() => ({ legacyMigrationNotice: vi.fn() }))
const toast = vi.fn()

vi.mock('@/api/backend', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/backend')>()),
  ...backend,
}))

vi.mock('@/components/ui/toast/use-toast', () => ({
  useToast: () => ({ toast }),
}))

const notice = (os: LegacyMigrationNotice['os']): LegacyMigrationNotice => ({
  source: '/datos/cuenta-cobro/database.sqlite',
  migratedAtUnix: 1_793_000_000,
  os,
})

const mountLayout = async () => {
  const pinia = createPinia()
  setActivePinia(pinia)
  const checkOnStartup = vi.spyOn(useUpdaterStore(), 'checkOnStartup').mockResolvedValue()
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { render: () => h('h1', 'Resumen') } }],
  })
  await router.push('/')
  await router.isReady()
  const wrapper = mount(MainLayout, { global: { plugins: [pinia, router] } })
  await flushPromises()
  return { wrapper, checkOnStartup }
}

describe('MainLayout', () => {
  beforeEach(() => {
    toast.mockReset()
    backend.legacyMigrationNotice.mockReset().mockResolvedValue(null)
  })

  it('al montar pide el aviso de migración y el chequeo de actualizaciones una sola vez', async () => {
    const { checkOnStartup } = await mountLayout()

    expect(backend.legacyMigrationNotice).toHaveBeenCalledTimes(1)
    expect(checkOnStartup).toHaveBeenCalledTimes(1)
  })

  it('sin migración pendiente no muestra ningún aviso', async () => {
    await mountLayout()

    expect(toast).not.toHaveBeenCalled()
  })

  it('tras importar los datos de la 1.x muestra el aviso «Datos importados»', async () => {
    backend.legacyMigrationNotice.mockResolvedValue(notice('macos'))

    await mountLayout()

    expect(toast).toHaveBeenCalledTimes(1)
    const [{ title, description, duration }] = toast.mock.calls[0] as [
      { title: string; description: string; duration?: number },
    ]
    expect(title).toBe('Datos importados')
    expect(description).toContain('No sigas usando CuentaCobro 1.x')
    expect(description).not.toContain('desinstalarla')
    // El aviso llega una sola vez: no puede cerrarse solo a los 5 s por defecto.
    expect(duration).toBe(Number.POSITIVE_INFINITY)
  })

  it('en Windows el aviso explica que se puede desinstalar la versión anterior', async () => {
    backend.legacyMigrationNotice.mockResolvedValue(notice('windows'))

    await mountLayout()

    const [{ description }] = toast.mock.calls[0] as [{ description: string }]
    expect(description).toContain('Configuración › Aplicaciones')
    expect(description).toContain('desinstalarla')
  })
})
