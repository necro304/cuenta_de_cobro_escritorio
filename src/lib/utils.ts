import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Extrae un mensaje legible de un error desconocido capturado en un catch. */
export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return String(error)
}
