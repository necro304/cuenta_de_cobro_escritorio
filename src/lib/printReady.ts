import { nextTick } from 'vue'
import { notifyPrintReady } from '@/api/backend'

/** Margen para que el motor termine el layout tras el último render. */
const SETTLE_DELAY_MS = 100

/**
 * Para las vistas imprimibles en modo `?pdf=1` (ventana oculta de exportación):
 * espera DOM, fuentes e imágenes (firma en data URL) y avisa al backend que ya
 * puede capturar el PDF.
 */
export async function signalPrintReady(): Promise<void> {
  await nextTick()
  if (document.fonts) await document.fonts.ready
  const pendingImages = Array.from(document.querySelectorAll('img')).filter(
    (image) => !image.complete && typeof image.decode === 'function',
  )
  await Promise.all(pendingImages.map((image) => image.decode().catch(() => undefined)))
  await new Promise((resolve) => setTimeout(resolve, SETTLE_DELAY_MS))
  await notifyPrintReady()
}
