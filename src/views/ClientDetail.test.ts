import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ClientDetail from './ClientDetail.vue'

const push = vi.fn()
const toast = vi.fn()
const exportClientReportPdf = vi.fn()

vi.mock('vue-router', () => ({
  RouterLink: {
    props: ['to'],
    template: '<a :href="String(to)"><slot /></a>',
  },
  useRoute: () => ({ params: { id: '7' } }),
  useRouter: () => ({ push }),
}))

vi.mock('vue-chartjs', () => ({
  Bar: { template: '<div data-test="activity-chart"></div>' },
}))

vi.mock('@/components/ui/toast/use-toast', () => ({
  useToast: () => ({ toast }),
}))

vi.mock('@/composables/useTheme', () => ({
  useTheme: () => ({ resolvedTheme: ref('light') }),
}))

describe('ClientDetail', () => {
  beforeEach(() => {
    push.mockReset()
    toast.mockReset()
    exportClientReportPdf.mockReset()
    exportClientReportPdf.mockResolvedValue({ success: true, message: '/tmp/informe.pdf' })

    const dbGet = vi.fn().mockResolvedValue({
      id: 7,
      name: 'Fundación Horizonte',
      document_id: '901234567-8',
      address: 'Carrera 12 # 34-56',
      city: 'Bogotá',
      phone: '601 555 0101',
      email: 'pagos@horizonte.co',
      created_at: '2026-01-10 10:00:00',
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
      value: { dbGet, dbQuery, exportClientReportPdf },
    })
  })

  it('presenta la cartera y el historial financiero del cliente', async () => {
    const wrapper = mount(ClientDetail)
    await flushPromises()

    expect(wrapper.text()).toContain('Fundación Horizonte')
    expect(wrapper.text()).toContain('$1.500.000')
    expect(wrapper.text()).toContain('$600.000')
    expect(wrapper.text()).toContain('$900.000')
    expect(wrapper.text()).toContain('40.0%')
    expect(wrapper.text()).toContain('Cuenta #24')
    expect(wrapper.find('[data-test="activity-chart"]').exists()).toBe(true)
  })

  it('abre una cuenta nueva con el cliente preseleccionado', async () => {
    const wrapper = mount(ClientDetail)
    await flushPromises()

    const newInvoiceButton = wrapper
      .findAll('button')
      .find((button) => button.text().includes('Nueva cuenta'))
    await newInvoiceButton?.trigger('click')

    expect(push).toHaveBeenCalledWith('/invoices/new?clientId=7')
  })

  it('solicita el informe PDF del cliente actual', async () => {
    const wrapper = mount(ClientDetail)
    await flushPromises()

    const reportButton = wrapper
      .findAll('button')
      .find((button) => button.text().includes('Informe PDF'))
    await reportButton?.trigger('click')
    await flushPromises()

    expect(exportClientReportPdf).toHaveBeenCalledWith({
      clientId: 7,
      clientName: 'Fundación Horizonte',
    })
    expect(toast).toHaveBeenCalledWith({
      title: 'Informe descargado',
      description: '/tmp/informe.pdf',
    })
  })
})
