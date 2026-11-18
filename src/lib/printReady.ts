import { nextTick } from 'vue'
import { notifyPrintReady } from '@/api/backend'

/** Margen para que el motor termine el layout tras el último render. */
const SETTLE_DELAY_MS = 100

/**
 * Para las vistas imprimibles en modo `?pdf=1` (ventana oculta de exportación):
 * espera DOM, fuentes e imágenes (firma en data URL) y avisa al backend que ya
 * puede capturar el PDF.
 */
export async function signalPrintReady(loadError?: string): Promise<void> {
  if (loadError) return notifyPrintReady(loadError)
  try {
    await nextTick()
    if (document.fonts) await document.fonts.ready
    const images = Array.from(document.querySelectorAll('img'))
    await Promise.all(images.map((image) => image.decode?.()))
  } catch {
    return notifyPrintReady('No se pudieron preparar los recursos del documento.')
  }
  await new Promise((resolve) => setTimeout(resolve, SETTLE_DELAY_MS))
  await notifyPrintReady()
}
