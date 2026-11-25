const menoresDeTreinta = [
  'CERO',
  'UN',
  'DOS',
  'TRES',
  'CUATRO',
  'CINCO',
  'SEIS',
  'SIETE',
  'OCHO',
  'NUEVE',
  'DIEZ',
  'ONCE',
  'DOCE',
  'TRECE',
  'CATORCE',
  'QUINCE',
  'DIECISÉIS',
  'DIECISIETE',
  'DIECIOCHO',
  'DIECINUEVE',
  'VEINTE',
  'VEINTIÚN',
  'VEINTIDÓS',
  'VEINTITRÉS',
  'VEINTICUATRO',
  'VEINTICINCO',
  'VEINTISÉIS',
  'VEINTISIETE',
  'VEINTIOCHO',
  'VEINTINUEVE',
]
const decenas = [
  '',
  '',
  '',
  'TREINTA',
  'CUARENTA',
  'CINCUENTA',
  'SESENTA',
  'SETENTA',
  'OCHENTA',
  'NOVENTA',
]
const centenas = [
  '',
  'CIENTO',
  'DOSCIENTOS',
  'TRESCIENTOS',
  'CUATROCIENTOS',
  'QUINIENTOS',
  'SEISCIENTOS',
  'SETECIENTOS',
  'OCHOCIENTOS',
  'NOVECIENTOS',
]
const escalas = [
  { divisor: 1_000_000_000_000, singular: 'UN BILLÓN', plural: 'BILLONES' },
  { divisor: 1_000_000, singular: 'UN MILLÓN', plural: 'MILLONES' },
  { divisor: 1_000, singular: 'MIL', plural: 'MIL' },
]

function enteroEnLetras(numero: number): string {
  for (const { divisor, singular, plural } of escalas) {
    if (numero < divisor) continue
    const grupos = Math.floor(numero / divisor)
    const resto = numero % divisor
    const prefijo = grupos === 1 ? singular : `${enteroEnLetras(grupos)} ${plural}`
    return resto ? `${prefijo} ${enteroEnLetras(resto)}` : prefijo
  }
  if (numero === 100) return 'CIEN'
  if (numero >= 100) {
    const centena = centenas[Math.floor(numero / 100)]
    const resto = numero % 100
    return resto ? `${centena} ${enteroEnLetras(resto)}` : centena
  }
  if (numero < 30) return menoresDeTreinta[numero]
  const decena = decenas[Math.floor(numero / 10)]
  const unidades = numero % 10
  return unidades ? `${decena} Y ${menoresDeTreinta[unidades]}` : decena
}

export function importeEnLetras(totalCentavos: bigint): string {
  const pesos = Number(totalCentavos / 100n)
  const centavos = Number(totalCentavos % 100n)
  const moneda = pesos === 1 ? 'PESO' : 'PESOS'
  const enlace = pesos > 0 && pesos % 1_000_000 === 0 ? ' DE ' : ' '
  const importe = `${enteroEnLetras(pesos)}${enlace}${moneda}`
  if (centavos === 0) return importe
  return `${importe} CON ${enteroEnLetras(centavos)} ${centavos === 1 ? 'CENTAVO' : 'CENTAVOS'}`
}
