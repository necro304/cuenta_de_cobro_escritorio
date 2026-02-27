export interface Profile {
  id: number
  name: string
  document_id: string
  address: string
  phone: string
  email: string
  bank_info: string
  signature_path?: string
}

export interface Client {
  id: number
  name: string
  document_id: string
  address: string
  city?: string
  phone: string
  email: string
  created_at?: string
}

export interface Invoice {
  id: number
  number: number
  date: string
  client_id: number
  bank_account_id?: number
  total: number
  notes: string
  status: 'draft' | 'paid' | 'partially_paid'
  created_at?: string
  client_name?: string
  paid_amount?: number
  balance?: number
}

export interface BankAccount {
  id: number
  bank: string
  account_type: string
  account_number: string
  is_default: number
  created_at?: string
}

export interface InvoiceItem {
  id: number
  invoice_id: number
  description: string
  quantity: number
  price: number
}

export interface InvoicePayment {
  id: number
  invoice_id: number
  date: string
  amount: number
  notes: string
  created_at?: string
}
