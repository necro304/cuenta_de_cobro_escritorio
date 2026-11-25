import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { InvoiceItem } from '@/types'
import DefaultTemplate from './DefaultTemplate.vue'
import SimpleTemplate from './SimpleTemplate.vue'

function concepto(quantity: number, price: number, id = 1): InvoiceItem {
  return { id, invoice_id: 1, description: 'Consultoría', quantity, price }
}

function imprimir(total: number | undefined, items: InvoiceItem[]) {
  const props = {
    profile: {},
    client: {},
    invoice: { total },
    items,
    bankAccount: null,
    signature: null,
  }
  return {
    detalle: mount(DefaultTemplate, { props }),
    simple: mount(SimpleTemplate, { props }),
  }
}

describe('importes impresos en las cuentas de cobro', () => {
  it('redondea 1,005 × 3 a 3,02 tanto en el concepto como en el total', () => {
    const { detalle, simple } = imprimir(3.02, [concepto(1.005, 3)])
    expect(detalle.get('tbody td.total').text()).toBe('3,02')
    expect(detalle.get('tbody td.cantidad').text()).toBe('1,005')
    expect(detalle.get('tbody td.valor-unitario').text()).toBe('3,00')
    expect(detalle.get('.subtotal-line').text()).toBe('Subtotal: 3,02')
    expect(detalle.get('.total-line').text()).toBe('TOTAL: 3,02')
    expect(simple.get('.monto').text()).toBe('TRES PESOS CON DOS CENTAVOS (COP 3,02)')
  })

  it.each([
    [150_000.5, '150.000,50', 'CIENTO CINCUENTA MIL PESOS CON CINCUENTA CENTAVOS'],
    [1.01, '1,01', 'UN PESO CON UN CENTAVO'],
    [0.21, '0,21', 'CERO PESOS CON VEINTIÚN CENTAVOS'],
    [1_000_000, '1.000.000,00', 'UN MILLÓN DE PESOS'],
    [2_001_000, '2.001.000,00', 'DOS MILLONES MIL PESOS'],
    [1_021_000_000, '1.021.000.000,00', 'MIL VEINTIÚN MILLONES DE PESOS'],
    [
      90_071_992_547_409.9,
      '90.071.992.547.409,90',
      'NOVENTA BILLONES SETENTA Y UN MIL NOVECIENTOS NOVENTA Y DOS MILLONES QUINIENTOS CUARENTA Y SIETE MIL CUATROCIENTOS NUEVE PESOS CON NOVENTA CENTAVOS',
    ],
  ] as const)('conserva cifras y letras para %s', (total, cifras, letras) => {
    const { detalle, simple } = imprimir(total, [concepto(1, total)])
    expect(detalle.get('tbody td.total').text()).toBe(cifras)
    expect(detalle.get('.total-line').text()).toBe(`TOTAL: ${cifras}`)
    expect(simple.get('.monto').text()).toBe(`${letras} (COP ${cifras})`)
  })

  it('suma los conceptos ya redondeados y conserva el precio unitario pactado', () => {
    const { detalle, simple } = imprimir(0.03, [concepto(1, 0.015), concepto(1, 0.015, 2)])
    expect(detalle.get('tbody td.valor-unitario').text()).toBe('0,015')
    expect(detalle.findAll('tbody td.total').map((fila) => fila.text())).toEqual(['0,02', '0,02'])
    expect(detalle.get('.subtotal-line').text()).toBe('Subtotal: 0,04')
    expect(detalle.get('.total-line').text()).toBe('TOTAL: 0,04')
    expect(simple.get('.monto').text()).toBe('CERO PESOS CON CUATRO CENTAVOS (COP 0,04)')
  })

  it('conserva el total registrado cuando una cuenta histórica carece de conceptos', () => {
    const { detalle, simple } = imprimir(21.5, [])
    expect(detalle.get('.total-line').text()).toBe('TOTAL: 21,50')
    expect(simple.get('.monto').text()).toBe('VEINTIÚN PESOS CON CINCUENTA CENTAVOS (COP 21,50)')
  })

  it('conserva el total registrado si hay un concepto histórico inválido', () => {
    const { detalle, simple } = imprimir(52, [concepto(1, 51), concepto(-1, 1, 2)])
    expect(detalle.get('.total-line').text()).toBe('TOTAL: 52,00')
    expect(simple.get('.monto').text()).toBe('CINCUENTA Y DOS PESOS (COP 52,00)')
  })

  it('indica que el importe no está disponible cuando faltan el total y los conceptos', () => {
    const { detalle, simple } = imprimir(undefined, [])
    expect(detalle.get('.total-line').text()).toBe('TOTAL: —')
    expect(simple.get('.monto').text()).toBe('Importe no disponible')
  })
})
