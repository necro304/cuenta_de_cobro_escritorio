import js from '@eslint/js'
import prettier from 'eslint-config-prettier/flat'
import { defineConfig, globalIgnores } from 'eslint/config'
import pluginVue from 'eslint-plugin-vue'
import globals from 'globals'
import tseslint from 'typescript-eslint'

export default defineConfig(
  globalIgnores([
    'dist/**',
    'coverage/**',
    '.vitest/**',
    'src-tauri/target/**',
    'src-tauri/gen/**',
    'release/**',
    'dist-electron/**',
  ]),

  js.configs.recommended,
  tseslint.configs.recommended,
  // flat/recommended registra vue-eslint-parser para *.vue: debe ir DESPUÉS de typescript-eslint.
  pluginVue.configs['flat/recommended'],

  // <script lang="ts"> dentro de los SFC se parsea con el parser de typescript-eslint.
  {
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: ['.vue'],
        sourceType: 'module',
      },
    },
  },

  // Frontend (webview de Tauri): entorno navegador.
  {
    files: ['src/**/*.{ts,vue}'],
    languageOptions: {
      globals: { ...globals.browser },
    },
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'vue/multi-word-component-names': 'off',
    },
  },

  // Archivos de configuración ejecutados por Node (vite, vitest, eslint, prettier).
  {
    files: ['*.config.{js,ts,mjs,cjs}'],
    languageOptions: {
      globals: { ...globals.node },
    },
  },

  // Componentes shadcn-vue generados: se relajan reglas que chocan con el código generado.
  {
    files: ['src/components/ui/**/*.{ts,vue}'],
    rules: {
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': 'off',
      'vue/require-default-prop': 'off',
    },
  },

  // Siempre al final: desactiva las reglas de formato que pelean con Prettier.
  prettier,
)
