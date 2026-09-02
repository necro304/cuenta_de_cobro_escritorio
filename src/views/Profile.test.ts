import { config, flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Profile from './Profile.vue'
import type { Profile as ProfileData } from '@/types'

const loadProfile = vi.hoisted(() => vi.fn())
const recolorSignatureDataUrl = vi.hoisted(() => vi.fn())
const dbRun = vi.fn()
const dbQuery = vi.fn()
const toast = vi.fn()

vi.mock('@/composables/useProfile', () => ({
  useProfile: () => ({ loadProfile }),
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
    dbRun.mockReset().mockResolvedValue({ changes: 1, lastInsertRowid: 0 })
    dbQuery.mockReset().mockResolvedValue([])
    toast.mockReset()
    recolorSignatureDataUrl.mockReset().mockResolvedValue('data:image/png;base64,recolored')
    loadProfile.mockReset().mockResolvedValue({
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

    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: { dbRun, dbQuery },
    })
  })

  it('recolorea y persiste una firma existente al elegir otro tono', async () => {
    const wrapper = mount(Profile, {
      global: {
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
    expect(dbRun).toHaveBeenCalledWith(
      'UPDATE profile SET signature = ?, signature_color = ? WHERE id = 1',
      ['data:image/png;base64,recolored', '#1e3a8a'],
    )
    expect(wrapper.get('img[alt="Firma actual"]').attributes('src')).toBe(
      'data:image/png;base64,recolored',
    )
  })
})
