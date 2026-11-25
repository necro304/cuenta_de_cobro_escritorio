/** Formatea un monto como moneda colombiana (sin símbolo $). */
export const formatCurrency = (value: number | null | undefined, decimals = 0): string =>
  (value ?? 0).toLocaleString('es-CO', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })

/** Convierte una fecha ISO (yyyy-mm-dd) al formato colombiano dd/mm/yyyy. */
export const formatDate = (dateStr: string | undefined): string => {
  if (!dateStr) return ''
  const parts = dateStr.split('-')
  if (parts.length === 3) return `${parts[2]}/${parts[1]}/${parts[0]}`
  return dateStr
}

/** Fecha de hoy en formato ISO corto (yyyy-mm-dd), para inputs type="date". */
export const getTodayDate = (): string => {
  const today = new Date()
  const month = String(today.getMonth() + 1).padStart(2, '0')
  const day = String(today.getDate()).padStart(2, '0')
  return `${today.getFullYear()}-${month}-${day}`
}
