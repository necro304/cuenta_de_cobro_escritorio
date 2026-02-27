import js from '@eslint/js'
import ts from '@typescript-eslint/eslint-plugin'
import tsParser from '@typescript-eslint/parser'
import vuePlugin from 'eslint-plugin-vue'
import vueParser from 'vue-eslint-parser'
import globals from 'globals'

export default [
  { ignores: ['dist/**', 'dist-electron/**', 'release/**', 'node_modules/**'] },

  // CommonJS config files (tailwind.config.js, postcss.config.js)
  {
    files: ['*.config.js', '*.config.cjs'],
    languageOptions: { globals: { ...globals.node } },
  },

  // Electron main process (Node.js environment)
  {
    files: ['electron/**/*.ts', 'vite.config.ts'],
    languageOptions: {
      parser: tsParser,
      globals: { ...globals.node },
    },
    plugins: { '@typescript-eslint': ts },
    rules: {
      ...ts.configs.recommended.rules,
      '@typescript-eslint/no-explicit-any': 'error',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
    },
  },

  // Vue renderer process (Browser + Electron renderer)
  {
    files: ['src/**/*.ts'],
    languageOptions: {
      parser: tsParser,
      globals: { ...globals.browser },
    },
    plugins: { '@typescript-eslint': ts },
    rules: {
      ...ts.configs.recommended.rules,
      '@typescript-eslint/no-explicit-any': 'error',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
    },
  },

  // Vue SFC files
  {
    files: ['src/**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: { parser: tsParser, sourceType: 'module' },
      globals: { ...globals.browser },
    },
    plugins: { vue: vuePlugin, '@typescript-eslint': ts },
    rules: {
      ...vuePlugin.configs['vue3-essential'].rules,
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/no-explicit-any': 'error',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
    },
  },

  // Shadcn UI components — relax unused-vars (auto-generated code)
  {
    files: ['src/components/ui/**/*.vue', 'src/components/ui/**/*.ts'],
    rules: {
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': 'off',
    },
  },
]
