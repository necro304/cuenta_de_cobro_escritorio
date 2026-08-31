import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

const currentDirectory = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  build: {
    target: 'node20',
    outDir: 'dist-ssr',
    emptyOutDir: true,
    ssr: path.join(currentDirectory, 'cuentaDeCobro.test.ts'),
    rolldownOptions: {
      external: ['electron', 'better-sqlite3'],
      output: {
        entryFileNames: 'cuentaDeCobro.test.js',
      },
    },
  },
})
