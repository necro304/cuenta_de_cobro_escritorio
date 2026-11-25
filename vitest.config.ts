import { defineConfig, mergeConfig } from 'vitest/config'
import viteConfig from './vite.config.ts'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'happy-dom',
      include: ['src/**/*.{test,spec}.ts'],
      // Vitest 5 ya limpia el historial de mocks antes de cada test (clearMocks: true por defecto);
      // se deja explícito para documentar el comportamiento.
      clearMocks: true,
      // Restaura vi.spyOn y deshace vi.stubGlobal (p. ej. window.__TAURI_INTERNALS__) entre tests.
      restoreMocks: true,
      unstubGlobals: true,
    },
  }),
)
