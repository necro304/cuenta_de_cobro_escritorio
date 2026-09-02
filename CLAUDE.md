# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Proyecto

Aplicación de escritorio (Electron 41 + Vue 3 + TypeScript + Vite 8) para gestión de cuentas de cobro colombianas, con SQLite local (better-sqlite3), Tailwind CSS 4 y componentes Shadcn/Vue (reka-ui). UI y dominio en español.

Electron está fijado en 41.x a propósito: better-sqlite3 aún no compila ni publica prebuilds para el ABI de Electron 42 (v146). Antes de subir Electron, verificar que exista `better-sqlite3-*-electron-vXXX-*` en los releases de GitHub de better-sqlite3.

## Comandos

```bash
npm run dev          # Vite + Electron en modo desarrollo
npm run build        # vue-tsc + vite build + electron-builder (genera instaladores en release/)
npm run lint         # ESLint
npm run lint:fix     # ESLint con autofix
npm run format       # Prettier
npm run test:renderer # Componentes Vue con Vitest
npm run test:database # Integración del módulo de cuenta de cobro con Electron + SQLite temporal
npx vue-tsc --noEmit # Solo type-check (lo que corre CI)
```

El módulo de persistencia de cuentas de cobro tiene un harness de integración que corre dentro del proceso main de Electron contra SQLite temporal. CI (PRs a `main`) corre: lint, `vue-tsc --noEmit`, `npm run test:renderer`, `npm run test:database` y `npx vite build`. El trabajo diario va en la rama `dev`; los PRs apuntan a `main`. Los releases se publican con tags `v*` (matriz Linux/Windows/macOS hacia GitHub Releases del repo `necro304/cuenta_de_cobro_escritorio`, usado también por electron-updater para auto-actualización).

## Arquitectura

### Proceso main (`electron/`)

- `main.ts` — ventana, handlers IPC y electron-updater. Conserva handlers genéricos de SQL (`db-query`, `db-get`, `db-run`) para operaciones simples y expone `cuenta-de-cobro:open`/`cuenta-de-cobro:save` para la escritura atómica de cuentas de cobro.
- `cuentaDeCobro.ts` — módulo profundo de creación/edición: valida referencias y decimales, deriva total/estado, protege abonos y persiste cabecera + conceptos en una transacción. Importa el `db` existente; no duplicar estas reglas en el renderer.
- `cuentaDeCobro.test.ts` — harness de integración ejecutado con Electron y una base temporal; la interfaz `open`/`save` es la superficie de test.
- `database.ts` — abre `userData/database.sqlite` y `initDb()` crea el esquema: `profile` (singleton id=1), `clients`, `invoices`, `invoice_items`, `invoice_payments`, `bank_accounts`. Las **migraciones** son `ALTER TABLE` envueltos en try/catch (ignoran "columna ya existe") — sigue ese patrón para nuevas columnas.
- `preload.ts` — expone `window.electronAPI` vía contextBridge.
- `better-sqlite3` y `electron-updater` están como `external` (en `build.rolldownOptions` — Vite 8 usa Rolldown) en `vite.config.ts`: el main se emite como ESM y estos módulos CJS/nativos se resuelven desde `node_modules` en runtime. No importar better-sqlite3 en el renderer. El script `postinstall` (`electron-builder install-app-deps`) reconstruye better-sqlite3 para el ABI de Electron.

### Renderer (`src/`)

- El renderer ejecuta **SQL crudo** contra el main process: `window.electronAPI.dbQuery<T>(sql, params)`, `dbGet<T>`, `dbRun`. Tipos en `src/electron-api.d.ts`; params tipados como `DbParam = string | number | null | boolean`.
- `InvoiceEditor.vue` es la excepción intencional al SQL crudo: consume `window.electronAPI.cuentaDeCobro.open/save`. No escribir directamente en `invoices` ni `invoice_items` desde este flujo.
- Interfaces de dominio en `src/types/index.ts` (`Profile`, `Client`, `Invoice`, `InvoiceItem`, `InvoicePayment`, `BankAccount`). `Invoice.status` es `'draft' | 'paid' | 'partially_paid'`.
- Router con hash history (`src/router/index.ts`): las vistas principales viven bajo `MainLayout` (sidebar); `/print/:id` está **fuera** del layout para impresión limpia.
- Impresión/PDF: `InvoicePrint.vue` usa `window.print()` con `@media print`; las plantillas seleccionables viven en `src/components/templates/` (`DefaultTemplate`, `SimpleTemplate`). Para agregar una plantilla: crear el componente, registrarla en el `Select` de InvoicePrint.
- `src/lib/numeroALetras.ts` convierte montos a letras en español (requisito de la cuenta de cobro).
- Componentes UI Shadcn/Vue en `src/components/ui/`; alias `@` → `src/`. Iconos de `@lucide/vue` (no `lucide-vue-next`, que está deprecado).
- Tailwind 4 en modo compatibilidad: `src/assets/index.css` usa `@import 'tailwindcss'` + `@config '../../tailwind.config.js'` (el theme shadcn sigue viviendo en el config JS legacy).

## Convenciones

- Nunca usar `any` — usar tipos de `@/types` o genéricos; los dbQuery/dbGet siempre con genérico explícito.
- Errores al usuario con `toast({ variant: 'destructive' })`, nunca `console.error` en el renderer.
- Validación de formularios: función `validate()` que retorna `string | null` antes de guardar (ver `InvoiceEditor.vue`, `Clients.vue`).
- El perfil es un singleton: siempre `WHERE id = 1`.

## Agent skills

### Issue tracker

Los issues y specs se gestionan como archivos Markdown locales bajo `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Se usan las cinco etiquetas canónicas: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` y `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

El repositorio usa un esquema single-context. See `docs/agents/domain.md`.
