import type { ConceptoDeCobroDraft, Money } from '../types/cuentaDeCobro'

export interface ParsedDecimal {
  coefficient: bigint
  scale: number
  canonical: string
}

export const MAX_SAFE_CENTS = BigInt(Number.MAX_SAFE_INTEGER)

export const centsToMoney = (cents: bigint): Money => {
  const whole = cents / 100n
  const decimal = (cents % 100n).toString().padStart(2, '0')
  return `${whole}.${decimal}`
}

export const formatMoneyForDisplay = (money: Money): string => {
  const [whole, decimal] = money.split('.')
  return `${BigInt(whole).toLocaleString('es-CO')},${decimal}`
}

export const parseDecimal = (input: string, allowZero: boolean): ParsedDecimal | null => {
  const value = input.trim()
  const match = /^(\d+)(?:\.(\d+))?$/.exec(value)
  if (!match) return null

  const integerPart = match[1].replace(/^0+(?=\d)/, '')
  const fractionPart = (match[2] ?? '').replace(/0+$/, '')
  const coefficient = BigInt(`${integerPart}${fractionPart}`)
  if (coefficient === 0n && !allowZero) return null

  return {
    coefficient,
    scale: fractionPart.length,
    canonical: fractionPart ? `${integerPart}.${fractionPart}` : integerPart,
  }
}

export const roundProductToCents = (left: ParsedDecimal, right: ParsedDecimal): bigint | null => {
  const product = left.coefficient * right.coefficient
  const scale = left.scale + right.scale
  let cents: bigint

  if (scale <= 2) {
    cents = product * 10n ** BigInt(2 - scale)
  } else {
    const divisor = 10n ** BigInt(scale - 2)
    const remainder = product % divisor
    cents = product / divisor
    if (remainder * 2n >= divisor) cents += 1n
  }

  return cents <= MAX_SAFE_CENTS ? cents : null
}

const expandExponential = (value: number): string | null => {
  if (!Number.isFinite(value) || value < 0) return null

  const serialized = value.toString().toLowerCase()
  if (!serialized.includes('e')) return serialized

  const [mantissa, exponentText] = serialized.split('e')
  const exponent = Number(exponentText)
  if (!Number.isInteger(exponent)) return null

  const decimalIndex = mantissa.indexOf('.')
  const integerLength = decimalIndex === -1 ? mantissa.length : decimalIndex
  const digits = mantissa.replace('.', '')
  const expandedIndex = integerLength + exponent

  if (expandedIndex <= 0) return `0.${'0'.repeat(-expandedIndex)}${digits}`
  if (expandedIndex >= digits.length) return `${digits}${'0'.repeat(expandedIndex - digits.length)}`
  return `${digits.slice(0, expandedIndex)}.${digits.slice(expandedIndex)}`
}

export const databaseNumberToDecimal = (value: number): ParsedDecimal | null => {
  const expanded = expandExponential(value)
  return expanded === null ? null : parseDecimal(expanded, true)
}

export const decimalToDatabaseNumber = (decimal: ParsedDecimal): number | null => {
  const value = Number(decimal.canonical)
  const restored = databaseNumberToDecimal(value)
  return restored?.canonical === decimal.canonical ? value : null
}

export const databaseNumberToCents = (value: number): bigint | null => {
  const decimal = databaseNumberToDecimal(value)
  if (!decimal) return null

  return roundProductToCents(decimal, {
    coefficient: 1n,
    scale: 0,
    canonical: '1',
  })
}

export const centsToDatabaseNumber = (cents: bigint): number | null => {
  const decimal = parseDecimal(centsToMoney(cents), true)
  return decimal ? decimalToDatabaseNumber(decimal) : null
}

export const calculatePreviewTotal = (concepts: ConceptoDeCobroDraft[]): Money => {
  let totalCents = 0n

  for (const concept of concepts) {
    const quantity = parseDecimal(concept.quantity, false)
    const price = parseDecimal(concept.price, false)
    if (!quantity || !price) continue

    const subtotalCents = roundProductToCents(quantity, price)
    if (subtotalCents === null || totalCents + subtotalCents > MAX_SAFE_CENTS) continue
    totalCents += subtotalCents
  }

  return centsToMoney(totalCents)
}
