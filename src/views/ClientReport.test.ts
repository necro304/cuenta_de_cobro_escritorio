import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ClientReport from './ClientReport.vue'

const notifyPrintReady = vi.fn()

vi.mock('vue-router', () => ({
  useRoute: () => ({ params: { id: '7' }, query: { pdf: '1' } }),
}))

describe('ClientReport', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    notifyPrintReady.mockReset()

    const dbGet = vi.fn().mockImplementation((sql: string) => {
      if (sql.includes('FROM profile')) {
        return Promise.resolve({
          id: 1,
          name: 'María Rodríguez',
          document_id: '52.123.456',
          address: 'Bogotá',
          phone: '300 000 0000',
          email: 'maria@example.com',
          bank_info: '',
        })
      }
      return Promise.resolve({
        id: 7,
        name: 'Fundación Horizonte',
        document_id: '901234567-8',
        address: 'Carrera 12 # 34-56',
        city: 'Bogotá',
        phone: '601 555 0101',
        email: 'pagos@horizonte.co',
      })
    })

    const dbQuery = vi.fn().mockImplementation((sql: string) => {
      if (sql.includes('SELECT i.*')) {
        return Promise.resolve([
          {
            id: 11,
            number: 24,
            date: '2026-08-15',
            client_id: 7,
            total: 1_000_000,
            notes: '',
            status: 'partially_paid',
            paid_amount: 600_000,
            balance: 400_000,
          },
          {
            id: 10,
            number: 19,
            date: '2026-07-12',
            client_id: 7,
            total: 500_000,
            notes: '',
            status: 'draft',
            paid_amount: 0,
            balance: 500_000,
          },
        ])
      }
      if (sql.includes('FROM invoice_payments p') && sql.includes('LIMIT 8')) {
        return Promise.resolve([
          {
            id: 3,
            invoice_id: 11,
            invoice_number: 24,
            date: '2026-08-20',
            amount: 600_000,
            notes: 'Transferencia',
          },
        ])
      }
      return Promise.resolve([])
    })

    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: { dbGet, dbQuery, notifyPrintReady },
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
    expect(notifyPrintReady).toHaveBeenCalledOnce()
  })
})
