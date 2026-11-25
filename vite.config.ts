import { fileURLToPath, URL } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// Variables que inyecta la CLI de Tauri: https://v2.tauri.app/reference/environment-variables/
const host = process.env.TAURI_DEV_HOST
const isDebug = Boolean(process.env.TAURI_ENV_DEBUG)

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  // No limpiar la terminal: `tauri dev` imprime ahí los errores de compilación de Rust.
  clearScreen: false,
  server: {
    // Debe coincidir con build.devUrl de src-tauri/tauri.conf.json.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: {
      // El backend Rust lo vigila la CLI de Tauri, no Vite.
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    // Sin `target` explícito: el default de Vite 8 ('baseline-widely-available' =
    // chrome111 / edge111 / firefox114 / safari16.4) coincide con el mínimo que exige Tailwind 4.
    minify: !isDebug,
    sourcemap: isDebug,
  },
})
