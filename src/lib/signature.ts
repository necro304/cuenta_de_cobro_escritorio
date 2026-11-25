export const DEFAULT_SIGNATURE_COLOR = '#245f46'

export const SIGNATURE_COLOR_PRESETS = [
  { name: 'Negro', value: '#111827' },
  { name: 'Azul', value: '#1e3a8a' },
  { name: 'Verde', value: DEFAULT_SIGNATURE_COLOR },
  { name: 'Vino', value: '#7f1d1d' },
] as const

export const normalizeSignatureColor = (value: string): string | null =>
  /^#[\da-f]{6}$/i.test(value) ? value.toLowerCase() : null

const parseColor = (color: string): [number, number, number] => {
  const normalized = normalizeSignatureColor(color)
  if (!normalized) throw new Error('Color de firma inválido')

  return [
    Number.parseInt(normalized.slice(1, 3), 16),
    Number.parseInt(normalized.slice(3, 5), 16),
    Number.parseInt(normalized.slice(5, 7), 16),
  ]
}

export const recolorSignaturePixels = (
  pixels: Uint8ClampedArray,
  color: string,
): Uint8ClampedArray => {
  if (pixels.length % 4 !== 0) throw new Error('Datos de imagen inválidos')

  const [red, green, blue] = parseColor(color)
  const result = new Uint8ClampedArray(pixels)
  let visiblePixels = 0
  let opaqueLightPixels = 0

  for (let index = 0; index < pixels.length; index += 4) {
    const alpha = pixels[index + 3]
    if (alpha === 0) continue
    visiblePixels += 1

    const luminance =
      pixels[index] * 0.2126 + pixels[index + 1] * 0.7152 + pixels[index + 2] * 0.0722
    if (alpha > 250 && luminance > 245) opaqueLightPixels += 1
  }

  const hasLightBackground = visiblePixels > 0 && opaqueLightPixels / visiblePixels > 0.25

  for (let index = 0; index < result.length; index += 4) {
    const alpha = pixels[index + 3]
    if (alpha === 0) continue

    result[index] = red
    result[index + 1] = green
    result[index + 2] = blue

    if (hasLightBackground) {
      const luminance =
        pixels[index] * 0.2126 + pixels[index + 1] * 0.7152 + pixels[index + 2] * 0.0722
      result[index + 3] = Math.round(alpha * (1 - luminance / 255))
    }
  }

  return result
}

export const recolorSignatureDataUrl = (dataUrl: string, color: string): Promise<string> => {
  const normalizedColor = normalizeSignatureColor(color)
  if (!normalizedColor) return Promise.reject(new Error('Color de firma inválido'))

  return new Promise((resolve, reject) => {
    const image = new Image()
    image.onload = () => {
      const canvas = document.createElement('canvas')
      const scale = Math.min(1, 1600 / Math.max(image.naturalWidth, image.naturalHeight))
      canvas.width = Math.max(1, Math.round(image.naturalWidth * scale))
      canvas.height = Math.max(1, Math.round(image.naturalHeight * scale))
      const context = canvas.getContext('2d')
      if (!context) {
        reject(new Error('No se pudo procesar la firma'))
        return
      }

      context.drawImage(image, 0, 0)
      const imageData = context.getImageData(0, 0, canvas.width, canvas.height)
      imageData.data.set(recolorSignaturePixels(imageData.data, normalizedColor))
      context.putImageData(imageData, 0, 0)
      resolve(canvas.toDataURL('image/png'))
    }
    image.onerror = () => reject(new Error('No se pudo cargar la firma'))
    image.src = dataUrl
  })
}
