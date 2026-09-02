import { config, flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import InvoiceEditor from './InvoiceEditor.vue'
import type { CuentaDeCobroModule, SaveCuentaDeCobroCommand } from '@/types/cuentaDeCobro'

const push = vi.fn()
const toast = vi.fn()
const save = vi.fn()

vi.mock('vue-router', () => ({
  useRoute: () => ({ params: {} }),
  useRouter: () => ({ push }),
}))

vi.mock('@/components/ui/toast/use-toast', () => ({
  useToast: () => ({ toast }),
}))

config.global.renderStubDefaultSlot = true

describe('InvoiceEditor', () => {
  beforeEach(() => {
    push.mockReset()
    toast.mockReset()
    save.mockReset()
    save.mockResolvedValue({
      ok: true,
      value: { id: 1, number: 1, total: '100.00', status: 'draft' },
    })

    const cuentaDeCobro: CuentaDeCobroModule = {
      open: vi.fn().mockResolvedValue({
        ok: true,
        value: {
          target: { kind: 'create' },
          cuenta: {
            number: '1',
            date: '2026-09-02',
            clientId: 1,
            bankAccountId: 1,
            notes: '',
            concepts: [{ description: 'Servicio', quantity: '1', price: '100' }],
          },
          clientes: [{ id: 1, name: 'Cliente de prueba' }],
          cuentasBancarias: [
            {
              id: 1,
              bank: 'Banco de prueba',
              accountType: 'Ahorros',
              accountNumber: '1234',
              isDefault: true,
            },
          ],
          summary: {
            total: '0.00',
            paidAmount: '0.00',
            balance: '0.00',
            paymentStatus: 'draft',
          },
          diagnostics: [],
        },
      }),
      save,
    }

    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: { cuentaDeCobro },
    })
  })

  const mountEditor = async () => {
    const wrapper = mount(InvoiceEditor, {
      global: {
        stubs: {
          PageHeader: true,
          Popover: true,
          PopoverTrigger: true,
          PopoverContent: true,
          Command: true,
          CommandInput: true,
          CommandEmpty: true,
          CommandList: true,
          CommandGroup: true,
          CommandItem: true,
          Select: true,
          SelectTrigger: true,
          SelectValue: true,
          SelectContent: true,
          SelectItem: true,
        },
      },
    })
    await flushPromises()
    return wrapper
  }

  it.each([
    { field: 'cantidad', inputIndex: 0, value: '2', subtotal: '200,00' },
    { field: 'precio unitario', inputIndex: 1, value: '1500', subtotal: '1.500,00' },
  ])(
    'mantiene visible $field y actualiza el subtotal al escribir',
    async ({ inputIndex, value, subtotal }) => {
      const wrapper = await mountEditor()
      const decimalInputs = wrapper.findAll('input[step="0.01"]')
      expect(decimalInputs).toHaveLength(2)

      await decimalInputs[inputIndex].setValue(value)

      expect(wrapper.findAll('input[step="0.01"]')).toHaveLength(2)
      const subtotalText = wrapper
        .findAll('p')
        .find((paragraph) => paragraph.text().includes('Subtotal'))
      expect(subtotalText?.text()).toContain(`$${subtotal}`)
    },
  )

  it('conserva el número de la cuenta como texto al guardar', async () => {
    const wrapper = await mountEditor()

    await wrapper.get('#invoice-number').setValue('7')
    const saveButton = wrapper
      .findAll('button')
      .find((button) => button.text() === 'Guardar cuenta')
    expect(saveButton).toBeDefined()
    await saveButton?.trigger('click')
    await flushPromises()

    const command = save.mock.calls[0][0] as SaveCuentaDeCobroCommand
    expect(command.cuenta.number).toBe('7')
  })
})
