import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ClientReport from './ClientReport.vue'

const backend = vi.hoisted(() => ({
  readClientReportData: vi.fn(),
  notifyPrintReady: vi.fn(),
}))

vi.mock('@/api/backend', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/backend')>()),
  ...backend,
}))

vi.mock('vue-router', () => ({
  useRoute: () => ({ params: { id: '7' }, query: { pdf: '1' } }),
}))

describe('ClientReport', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    backend.notifyPrintReady.mockReset().mockResolvedValue(undefined)

    backend.readClientReportData.mockReset().mockResolvedValue({
      profile: { name: 'María Rodríguez' },
      client: {
        name: 'Fundación Horizonte',
        document_id: '901234567-8',
        city: 'Bogotá',
        phone: '601 555 0101',
        email: 'pagos@horizonte.co',
      },
      invoices: [
        {
          id: 11,
          number: 24,
          date: '2026-08-15',
          total: 1_000_000,
          status: 'partially_paid',
          paid_amount: 600_000,
          balance: 400_000,
        },
        {
          id: 10,
          number: 19,
          date: '2026-07-12',
          total: 500_000,
          status: 'draft',
          paid_amount: 0,
          balance: 500_000,
        },
      ],
      payments: [
        {
          id: 3,
          invoice_number: 24,
          date: '2026-08-20',
          amount: 600_000,
          notes: 'Transferencia',
        },
      ],
      monthlyInvoices: [],
      monthlyPayments: [],
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('compone el informe financiero y avisa cuando está listo para PDF', async () => {
    const wrapper = mount(ClientReport)
    await flushPromises()
    await vi.runAllTimersAsync()

    expect(wrapper.text()).toContain('Informe de relación comercial')
    expect(wrapper.text()).toContain('Fundación Horizonte')
    expect(wrapper.text()).toContain('$1.500.000')
    expect(wrapper.text()).toContain('$900.000')
    expect(wrapper.text()).toContain('40%')
    expect(wrapper.text()).toContain('Cuenta #24')
    expect(wrapper.findAll('.report-page')).toHaveLength(2)
    expect(backend.readClientReportData).toHaveBeenCalledWith(7)
    expect(backend.notifyPrintReady.mock.calls).toEqual([[]])
  })

  it('comunica el fallo de carga sin autorizar la impresión del mensaje de error', async () => {
    backend.readClientReportData.mockRejectedValueOnce(new Error('Falló la consulta'))
    const wrapper = mount(ClientReport)
    await flushPromises()
    await vi.runAllTimersAsync()

    expect(wrapper.find('.report-document').exists()).toBe(false)
    expect(backend.notifyPrintReady.mock.calls).toEqual([
      ['No se pudo cargar el informe del cliente.'],
    ])
  })
})
