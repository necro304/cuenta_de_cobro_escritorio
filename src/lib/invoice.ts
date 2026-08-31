import type { Invoice } from '@/types'

export type InvoiceStatus = Invoice['status']

/** Saldo pendiente de una cuenta de cobro (nunca negativo). */
export const calculateBalance = (total: number, paidAmount = 0): number =>
  Math.max(0, total - paidAmount)

/** Determina el estado de una cuenta de cobro según el monto pagado. */
export const determineStatus = (total: number, paidAmount = 0): InvoiceStatus => {
  if (paidAmount >= total) return 'paid'
  if (paidAmount > 0) return 'partially_paid'
  return 'draft'
}

/** Etiquetas en español para cada estado. */
export const INVOICE_STATUS_LABELS: Record<InvoiceStatus, string> = {
  paid: 'Pagada',
  partially_paid: 'Abonada',
  draft: 'Pendiente',
}

/** Opciones de estado para selects, en orden de captura. */
export const INVOICE_STATUS_OPTIONS: { value: InvoiceStatus; label: string }[] = [
  { value: 'draft', label: 'Pendiente' },
  { value: 'partially_paid', label: 'Abonada' },
  { value: 'paid', label: 'Pagada' },
]
