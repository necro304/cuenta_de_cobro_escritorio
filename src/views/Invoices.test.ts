import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Invoices from './Invoices.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import type { CuentaDeCobroSummary } from '@/types/cuentaDeCobro'

const backend = vi.hoisted(() => ({
  invoiceLedger: vi.fn(),
  invoicePaymentHistory: vi.fn(),
  saveInvoicePayment: vi.fn(),
  deleteInvoicePayment: vi.fn(),
  dbGet: vi.fn(),
}))
const toast = vi.hoisted(() => vi.fn())
vi.mock('@/api/backend', async (load) => ({
  ...(await load<typeof import('@/api/backend')>()),
  ...backend,
}))
vi.mock('@/components/ui/toast/use-toast', () => ({ useToast: () => ({ toast }) }))

const partial: CuentaDeCobroSummary = {
  total: '10.30',
  balance: '0.10',
  paidAmount: '10.20',
  paymentStatus: 'partially_paid',
}
const paid: CuentaDeCobroSummary = {
  ...partial,
  balance: '0.00',
  paidAmount: '10.30',
  paymentStatus: 'paid',
}
const payment = { id: 7, invoice_id: 2, amount: 10.2, date: '2026-11-30', notes: '' }

const openPayments = async () => {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: Invoices }],
  })
  await router.push('/')
  const wrapper = mount(Invoices, {
    global: {
      plugins: [createPinia(), router],
      stubs: {
        PageHeader: true,
        ConfirmDialog: true,
        SignatureAskDialog: true,
        Dialog: { props: ['open'], template: '<div v-if="open"><slot /></div>' },
        DialogContent: { template: '<section><slot /></section>' },
        DialogHeader: true,
        DialogFooter: true,
      },
    },
  })
  await flushPromises()
  await wrapper.get('[aria-label="Gestionar abonos de la cuenta 12"]').trigger('click')
  await flushPromises()
  return wrapper
}

describe('Abonos de la cuenta', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    backend.invoiceLedger.mockResolvedValue([
      { id: 2, number: 12, date: '2026-11-30', clientName: 'Sol', summary: { ...partial } },
    ])
    backend.invoicePaymentHistory.mockResolvedValue({ summary: partial, payments: [payment] })
  })

  it('bloquea un doble envío y permite reintentar solo la consulta si falla tras guardar', async () => {
    let finish!: (value: CuentaDeCobroSummary) => void
    backend.saveInvoicePayment.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve
      }),
    )
    const wrapper = await openPayments()
    expect(wrapper.get<HTMLInputElement>('#payment-amount').element.value).toBe('0.10')
    await wrapper.get('#payment-date').setValue('2026-11-30')
    const button = wrapper.findAll('button').find((b) => b.text() === 'Guardar abono')!
    const first = button.trigger('click')
    const second = button.trigger('click')
    await Promise.all([first, second])
    expect(backend.saveInvoicePayment).toHaveBeenCalledTimes(1)
    expect(backend.saveInvoicePayment).toHaveBeenCalledWith({
      invoiceId: 2,
      amount: '0.10',
      date: '2026-11-30',
      notes: '',
    })
    expect(button.attributes('disabled')).toBeDefined()
    backend.invoicePaymentHistory.mockRejectedValueOnce(new Error('consulta fallida'))
    finish(paid)
    await flushPromises()
    expect(toast).toHaveBeenCalledWith({ title: 'Abono registrado' })
    expect(toast).not.toHaveBeenCalledWith(expect.objectContaining({ variant: 'destructive' }))
    expect(wrapper.get('[role="alert"]').text()).toContain('No se pudo actualizar el historial')
    expect(wrapper.text()).toContain('Pagada')
    backend.invoicePaymentHistory.mockResolvedValue({ summary: paid, payments: [payment] })
    await wrapper.get('[role="alert"] button').trigger('click')
    await flushPromises()
    expect(wrapper.find('[role="alert"]').exists()).toBe(false)
    expect(backend.saveInvoicePayment).toHaveBeenCalledTimes(1)
    expect(wrapper.find('#payment-amount').exists()).toBe(false)
  })

  it('conserva el formulario ante un error previo al commit', async () => {
    backend.saveInvoicePayment.mockRejectedValue(new Error('Fecha no válida'))
    const wrapper = await openPayments()
    await wrapper.get('#payment-notes').setValue('referencia 19')
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Guardar abono')!
      .trigger('click')
    await flushPromises()
    expect(toast).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'No se pudo registrar el abono',
        description: 'Fecha no válida',
      }),
    )
    expect(wrapper.get<HTMLInputElement>('#payment-notes').element.value).toBe('referencia 19')
    expect(backend.invoicePaymentHistory).toHaveBeenCalledTimes(1)
  })

  it('elimina con el comando transaccional y conserva la confirmación si falla el refresco', async () => {
    const wrapper = await openPayments()
    backend.deleteInvoicePayment.mockResolvedValue({
      ...partial,
      paidAmount: '0.00',
      balance: '10.30',
      paymentStatus: 'draft',
    })
    backend.invoicePaymentHistory.mockRejectedValueOnce(new Error('consulta fallida'))
    await wrapper.get('[aria-label="Eliminar abono"]').trigger('click')
    const dialog = wrapper.getComponent(ConfirmDialog)
    dialog.vm.$emit('confirm')
    dialog.vm.$emit('confirm')
    await flushPromises()
    expect(backend.deleteInvoicePayment).toHaveBeenCalledTimes(1)
    expect(backend.deleteInvoicePayment).toHaveBeenCalledWith(2, 7)
    expect(toast).toHaveBeenCalledWith({ title: 'Abono eliminado' })
    expect(dialog.props('open')).toBe(false)
    expect(wrapper.get('[role="alert"]').text()).toContain('historial')
  })
})
