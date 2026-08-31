import { useColorMode } from '@vueuse/core'

export type ThemeMode = 'auto' | 'light' | 'dark'

/**
 * Tema de la app: 'auto' sigue al sistema en vivo; 'light'/'dark' lo fuerzan.
 * Persiste en localStorage['theme'] y aplica la clase resuelta al <html>.
 */
export function useTheme() {
  const theme = useColorMode({ emitAuto: true, storageKey: 'theme' })

  // Tema efectivo en pantalla: 'auto' ya resuelto a 'light' | 'dark'
  const resolvedTheme = theme.state

  const setTheme = (value: ThemeMode) => {
    theme.value = value
  }

  return { theme, resolvedTheme, setTheme }
}
