import { config, flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import InvoicePrint from './InvoicePrint.vue'
import type { Profile } from '@/types'

const toast = vi.fn()
const routeQuery: Record<string, string | undefined> = {}
const backend = vi.hoisted(() => ({
  readInvoicePrintData: vi.fn(),
  exportInvoicePdf: vi.fn(),
  notifyPrintReady: vi.fn(),
}))

vi.mock('@/api/backend', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/backend')>()),
  ...backend,
}))

vi.mock('vue-router', () => ({
  useRoute: () => ({ params: { id: '11' }, query: routeQuery }),
  useRouter: () => ({ push: vi.fn() }),
}))

vi.mock('@/components/ui/toast/use-toast', () => ({
  useToast: () => ({ toast }),
}))

config.global.renderStubDefaultSlot = true

const profileRow: Profile = {
  id: 1,
  name: 'María Rodríguez',
  document_id: '52.123.456',
  address: 'Bogotá',
  phone: '',
  email: '',
  bank_info: '',
  signature: 'data:image/png;base64,firma',
  signature_mode: 'auto',
  default_template: 'simple',
}

const mountView = () =>
  mount(InvoicePrint, {
    global: {
      plugins: [createPinia()],
      stubs: {
        Select: true,
        SelectContent: true,
        SelectGroup: true,
        SelectItem: true,
        SelectTrigger: true,
        SelectValue: true,
        SignatureAskDialog: true,
        DefaultTemplate: { props: ['signature'], template: '<div data-test="default" />' },
        SimpleTemplate: {
          props: ['signature'],
          template: '<div data-test="simple" :data-signature="signature ?? \'\'" />',
        },
      },
    },
  })

describe('InvoicePrint', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    toast.mockReset()
    delete routeQuery.pdf
    delete routeQuery.template
    delete routeQuery.signature
    backend.notifyPrintReady.mockReset().mockResolvedValue(undefined)
    backend.exportInvoicePdf
      .mockReset()
      .mockResolvedValue({ status: 'done', message: '/tmp/c.pdf' })
    backend.readInvoicePrintData.mockReset().mockResolvedValue({
      profile: profileRow,
      client: { name: 'Fundación Horizonte' },
      invoice: { id: 11, number: 24, date: '2026-08-15', total: 1 },
      bankAccount: null,
      items: [],
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('usa la plantilla y la firma del perfil y exporta con ellas', async () => {
    const wrapper = mountView()
    await flushPromises()

    expect(wrapper.get('[data-test="simple"]').attributes('data-signature')).toBe(
      'data:image/png;base64,firma',
    )
    const pdfButton = wrapper.findAll('button').find((b) => b.text().includes('Descargar PDF'))
    await pdfButton?.trigger('click')
    await flushPromises()

    expect(backend.exportInvoicePdf).toHaveBeenCalledWith({
      invoiceId: 11,
      invoiceNumber: 24,
      template: 'simple',
      includeSignature: true,
    })
    expect(toast).toHaveBeenCalledWith({ title: 'PDF descargado', description: '/tmp/c.pdf' })
    expect(backend.notifyPrintReady).not.toHaveBeenCalled()
  })

  it('en modo PDF respeta la query y avisa print-ready una sola vez', async () => {
    Object.assign(routeQuery, { pdf: '1', template: 'default', signature: '0' })
    const wrapper = mountView()
    await flushPromises()
    await vi.runAllTimersAsync()

    expect(wrapper.find('[data-test="default"]').exists()).toBe(true)
    expect(wrapper.text()).not.toContain('Descargar PDF')
    expect(backend.readInvoicePrintData).toHaveBeenCalledWith(11)
    expect(backend.notifyPrintReady.mock.calls).toEqual([[]])
  })

  it('comunica el fallo de carga y nunca anuncia una cuenta imprimible', async () => {
    routeQuery.pdf = '1'
    backend.readInvoicePrintData.mockRejectedValueOnce(new Error('Falló la consulta'))
    const wrapper = mountView()
    await flushPromises()
    await vi.runAllTimersAsync()

    expect(wrapper.find('[data-test="simple"]').exists()).toBe(false)
    expect(backend.notifyPrintReady.mock.calls).toEqual([['No se pudo cargar la cuenta de cobro.']])
  })
})
