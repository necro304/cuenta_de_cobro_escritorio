import { describe, expect, it } from 'vitest'
import { normalizeSignatureColor, recolorSignaturePixels } from './signature'

describe('signature', () => {
  it('normaliza colores hexadecimales válidos', () => {
    expect(normalizeSignatureColor('#1E3A8A')).toBe('#1e3a8a')
    expect(normalizeSignatureColor('blue')).toBeNull()
  })

  it('recolorea una firma transparente conservando su opacidad', () => {
    const pixels = new Uint8ClampedArray([36, 95, 70, 128, 0, 0, 0, 0])

    expect([...recolorSignaturePixels(pixels, '#1e3a8a')]).toEqual([30, 58, 138, 128, 0, 0, 0, 0])
  })

  it('elimina el fondo blanco de una firma subida antes de recolorearla', () => {
    const pixels = new Uint8ClampedArray([
      255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
    ])

    expect([...recolorSignaturePixels(pixels, '#7f1d1d')]).toEqual([
      127, 29, 29, 0, 127, 29, 29, 0, 127, 29, 29, 0, 127, 29, 29, 255,
    ])
  })
})
