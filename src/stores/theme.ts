import { useColorMode } from '@vueuse/core'
import { defineStore } from 'pinia'
import { computed } from 'vue'

export type ThemeMode = 'auto' | 'light' | 'dark'
export type ResolvedTheme = 'light' | 'dark'

/**
 * Tema de la app: 'auto' sigue al sistema en vivo; 'light'/'dark' lo fuerzan.
 * Persiste en localStorage['theme'] y aplica la clase resuelta (`dark`) al <html>.
 */
export const useThemeStore = defineStore('theme', () => {
  const colorMode = useColorMode({ storageKey: 'theme' })

  /** Preferencia elegida, incluido 'auto'. */
  const mode = computed<ThemeMode>(() => colorMode.store.value)
  /** Tema efectivo en pantalla ('auto' ya resuelto). */
  const resolved = computed<ResolvedTheme>(() => colorMode.state.value)

  function setMode(value: ThemeMode): void {
    colorMode.store.value = value
  }

  return { mode, resolved, setMode }
})
