import { config, flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Profile from './Profile.vue'
import type { Profile as ProfileData } from '@/types'

const recolorSignatureDataUrl = vi.hoisted(() => vi.fn())
const backend = vi.hoisted(() => ({ dbGet: vi.fn(), dbQuery: vi.fn(), dbRun: vi.fn() }))
const toast = vi.fn()

vi.mock('@/api/backend', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/backend')>()),
  ...backend,
}))

vi.mock('@/lib/signature', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/signature')>()),
  recolorSignatureDataUrl,
}))

vi.mock('@/components/ui/toast/use-toast', () => ({
  useToast: () => ({ toast }),
}))

config.global.renderStubDefaultSlot = true

describe('Profile', () => {
  beforeEach(() => {
    backend.dbRun.mockReset().mockResolvedValue({ changes: 1, lastInsertRowid: 0 })
    backend.dbQuery.mockReset().mockResolvedValue([])
    toast.mockReset()
    recolorSignatureDataUrl.mockReset().mockResolvedValue('data:image/png;base64,recolored')
    backend.dbGet.mockReset().mockResolvedValue({
      id: 1,
      name: 'Persona de prueba',
      document_id: '1234',
      address: '',
      phone: '',
      email: '',
      bank_info: '',
      signature: 'data:image/png;base64,original',
      signature_color: '#245f46',
    } satisfies ProfileData)
  })

  it('recolorea y persiste una firma existente al elegir otro tono', async () => {
    const wrapper = mount(Profile, {
      global: {
        plugins: [createPinia()],
        stubs: {
          PageHeader: true,
          Dialog: true,
          DialogContent: true,
          DialogDescription: true,
          DialogFooter: true,
          DialogHeader: true,
          DialogTitle: true,
          Select: true,
          SelectContent: true,
          SelectItem: true,
          SelectTrigger: true,
          SelectValue: true,
          SignaturePadDialog: true,
          ConfirmDialog: true,
        },
      },
    })
    await flushPromises()

    await wrapper.get('[aria-label="Usar tinta azul"]').trigger('click')
    await flushPromises()

    expect(recolorSignatureDataUrl).toHaveBeenCalledWith(
      'data:image/png;base64,original',
      '#1e3a8a',
    )
    expect(backend.dbRun).toHaveBeenCalledWith(
      'UPDATE profile SET signature = ?, signature_color = ? WHERE id = 1',
      ['data:image/png;base64,recolored', '#1e3a8a'],
    )
    expect(wrapper.get('img[alt="Firma actual"]').attributes('src')).toBe(
      'data:image/png;base64,recolored',
    )
  })
})
