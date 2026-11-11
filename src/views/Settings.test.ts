import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Settings from './Settings.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import type { ActionOutcome } from '@/api/backend'

const backend = vi.hoisted(() => ({
  backupDatabase: vi.fn(),
  restoreDatabase: vi.fn(),
  dbRun: vi.fn(),
}))
const relaunch = vi.hoisted(() => vi.fn())
const toast = vi.fn()

vi.mock('@/api/backend', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/backend')>()),
  ...backend,
}))

vi.mock('@tauri-apps/plugin-process', () => ({ relaunch }))

vi.mock('@/components/ui/toast/use-toast', () => ({
  useToast: () => ({ toast }),
}))

const mountSettings = async () => {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: Settings }],
  })
  await router.push('/')
  await router.isReady()
  const wrapper = mount(Settings, {
    global: {
      plugins: [createPinia(), router],
      stubs: { PageHeader: true, ConfirmDialog: true },
    },
  })
  await flushPromises()
  return wrapper
}

const clickButton = async (wrapper: VueWrapper, text: string) => {
  const button = wrapper.findAll('button').find((b) => b.text().includes(text))
  if (!button) throw new Error(`No existe el botón «${text}»`)
  await button.trigger('click')
  await flushPromises()
}

/** Restaurar pide confirmación: se pulsa el botón y se confirma el diálogo. */
const restore = async (wrapper: VueWrapper) => {
  await clickButton(wrapper, 'Restaurar respaldo')
  wrapper.findComponent(ConfirmDialog).vm.$emit('confirm')
  await flushPromises()
}

describe('Settings', () => {
  beforeEach(() => {
    toast.mockReset()
    relaunch.mockReset().mockResolvedValue(undefined)
    backend.backupDatabase.mockReset()
    backend.restoreDatabase.mockReset()
    backend.dbRun.mockReset().mockResolvedValue({ changes: 0, lastInsertRowid: 0 })
  })

  it('respaldo: cancelar el diálogo no muestra ningún toast', async () => {
    backend.backupDatabase.mockResolvedValue({ status: 'cancelled' } satisfies ActionOutcome)
    const wrapper = await mountSettings()

    await clickButton(wrapper, 'Crear respaldo')

    expect(backend.backupDatabase).toHaveBeenCalledTimes(1)
    expect(toast).not.toHaveBeenCalled()
  })

  it('respaldo: un fallo se muestra con toast destructive y el mensaje de Rust', async () => {
    backend.backupDatabase.mockResolvedValue({
      status: 'failed',
      message: 'No space left on device',
    } satisfies ActionOutcome)
    const wrapper = await mountSettings()

    await clickButton(wrapper, 'Crear respaldo')

    expect(toast).toHaveBeenCalledWith(
      expect.objectContaining({
        description: 'No space left on device',
        variant: 'destructive',
      }),
    )
  })

  it('respaldo: el éxito se confirma con el mensaje de Rust', async () => {
    backend.backupDatabase.mockResolvedValue({
      status: 'done',
      message: 'Respaldo guardado exitosamente.',
    } satisfies ActionOutcome)
    const wrapper = await mountSettings()

    await clickButton(wrapper, 'Crear respaldo')

    expect(toast).toHaveBeenCalledWith({
      title: 'Respaldo exitoso',
      description: 'Respaldo guardado exitosamente.',
    })
  })

  it('restaurar: cancelar el diálogo no muestra toast ni reinicia', async () => {
    backend.restoreDatabase.mockResolvedValue({ status: 'cancelled' } satisfies ActionOutcome)
    const wrapper = await mountSettings()

    await restore(wrapper)

    expect(backend.restoreDatabase).toHaveBeenCalledTimes(1)
    expect(toast).not.toHaveBeenCalled()
    expect(relaunch).not.toHaveBeenCalled()
  })

  it('restaurar: un fallo se muestra con toast destructive y no reinicia', async () => {
    backend.restoreDatabase.mockResolvedValue({
      status: 'failed',
      message: 'file is not a database',
    } satisfies ActionOutcome)
    const wrapper = await mountSettings()

    await restore(wrapper)

    expect(toast).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Error al restaurar',
        description: 'file is not a database',
        variant: 'destructive',
      }),
    )
    expect(relaunch).not.toHaveBeenCalled()
  })

  it('restaurar: si Rust respondiera éxito sin reiniciar, la vista llama a relaunch()', async () => {
    backend.restoreDatabase.mockResolvedValue({
      status: 'done',
      message: 'Restauración completa. Reiniciando...',
    } satisfies ActionOutcome)
    const wrapper = await mountSettings()

    await restore(wrapper)

    expect(relaunch).toHaveBeenCalledTimes(1)
  })
})
