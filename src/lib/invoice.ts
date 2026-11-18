import type { Invoice } from '@/types'

export type InvoiceStatus = Invoice['status']

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
