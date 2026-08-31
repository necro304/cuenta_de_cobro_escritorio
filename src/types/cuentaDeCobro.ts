export type DecimalInput = string
export type Money = string
export type EstadoPago = 'draft' | 'partially_paid' | 'paid'

export interface ConceptoDeCobroDraft {
  description: string
  quantity: DecimalInput
  price: DecimalInput
}

export interface CuentaDeCobroDraft {
  number: string
  date: string
  clientId: number | null
  bankAccountId: number | null
  notes: string
  concepts: ConceptoDeCobroDraft[]
}

export type CuentaDeCobroTarget = { kind: 'create' } | { kind: 'edit'; id: number }

export interface ClienteOption {
  id: number
  name: string
}

export interface CuentaBancariaOption {
  id: number
  bank: string
  accountType: string
  accountNumber: string
  isDefault: boolean
}

export interface CuentaDeCobroSummary {
  total: Money
  paidAmount: Money
  balance: Money
  paymentStatus: EstadoPago
}

export type CuentaDeCobroDiagnostic =
  | { code: 'HISTORIC_NUMBER_CONFLICT'; number: number }
  | { code: 'MISSING_CLIENT_REFERENCE'; clientId: number }
  | { code: 'MISSING_BANK_ACCOUNT_REFERENCE'; bankAccountId: number | null }
  | { code: 'INVALID_PERSISTED_ACCOUNT' }
  | { code: 'INVALID_PERSISTED_CONCEPTS' }
  | { code: 'INVALID_PERSISTED_PAYMENTS' }

export interface CuentaDeCobroEditorModel {
  target: CuentaDeCobroTarget
  cuenta: CuentaDeCobroDraft
  clientes: ClienteOption[]
  cuentasBancarias: CuentaBancariaOption[]
  summary: CuentaDeCobroSummary
  diagnostics: CuentaDeCobroDiagnostic[]
}

export type CuentaDeCobroField =
  | 'number'
  | 'date'
  | 'clientId'
  | 'bankAccountId'
  | 'concepts'
  | `concepts.${number}.description`
  | `concepts.${number}.quantity`
  | `concepts.${number}.price`

export type CuentaDeCobroValidationCode =
  | 'REQUIRED'
  | 'INVALID_POSITIVE_INTEGER'
  | 'INVALID_DATE'
  | 'INVALID_POSITIVE_DECIMAL'
  | 'AT_LEAST_ONE_CONCEPT_REQUIRED'
  | 'SUBTOTAL_ROUNDS_TO_ZERO'
  | 'AMOUNT_OUT_OF_RANGE'

export interface CuentaDeCobroValidationIssue {
  field: CuentaDeCobroField
  code: CuentaDeCobroValidationCode
}

export type CuentaDeCobroError =
  | { code: 'VALIDATION_FAILED'; issues: CuentaDeCobroValidationIssue[] }
  | { code: 'CUENTA_DE_COBRO_NOT_FOUND'; id: number }
  | { code: 'CLIENTE_NOT_FOUND'; id: number }
  | { code: 'CUENTA_BANCARIA_NOT_FOUND'; id: number }
  | { code: 'NUMBER_IN_USE'; number: number; suggestedNumber: number | null }
  | { code: 'TOTAL_BELOW_PAID_AMOUNT'; total: Money; paidAmount: Money }
  | { code: 'STORAGE_FAILURE' }

export type CuentaDeCobroResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: CuentaDeCobroError }

export type SaveCuentaDeCobroCommand =
  | { kind: 'create'; cuenta: CuentaDeCobroDraft }
  | { kind: 'edit'; id: number; cuenta: CuentaDeCobroDraft }

export interface SaveCuentaDeCobroReceipt {
  id: number
  number: number
  total: Money
  status: EstadoPago
}

export interface CuentaDeCobroModule {
  open(target: CuentaDeCobroTarget): Promise<CuentaDeCobroResult<CuentaDeCobroEditorModel>>
  save(command: SaveCuentaDeCobroCommand): Promise<CuentaDeCobroResult<SaveCuentaDeCobroReceipt>>
}
