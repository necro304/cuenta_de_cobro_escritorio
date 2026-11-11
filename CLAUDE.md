# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Proyecto

Aplicación de escritorio (Tauri 2.12 + Vue 3.5 + TypeScript 6.0 + Vite 8 + Pinia 4 + Tailwind 4.3 CSS-first + componentes Shadcn/Vue sobre reka-ui) para gestión de cuentas de cobro colombianas. SQLite vive en el proceso Rust (rusqlite `bundled`, una sola conexión). UI y dominio en español. Versión 2.x; la 1.x era Electron.

Versiones fijadas a propósito:

- Solo Tauri 2 estable (crate `tauri` 2.12, CLI 2.12). Nunca Tauri 3 alpha.
- TypeScript se queda en `~6.0` (ADR R1): vue-tsc 3.3 se rompe con TS 7 y typescript-eslint exige `<6.1.0`.
- `@vueuse/core` se queda en 14 (R2): reka-ui 2.11 depende de `^14.1`; con la 15 habría dos copias.
- Rust de CI fijo en 1.99.1 (`ci.yml` y `release.yml`, R34). Se sube a mano, en un PR propio, porque un stable nuevo puede traer lints que rompan `clippy -D warnings`. El crate declara `rust-version = "1.90"`.

Las decisiones de la migración desde Electron y las divergencias aceptadas están en `docs/adr/0001-migracion-a-tauri.md`. Léelo antes de tocar versiones fijadas, el updater, el PDF o la BD.

## Comandos

```bash
npm run dev            # tauri dev: Vite en el puerto 1420 (dev:web) + la app Tauri
npm run dev:web        # solo Vite (lo lanza tauri dev); sin Tauri, el backend responde "solo escritorio"
npm run build          # tauri build firmado: exige TAURI_SIGNING_PRIVATE_KEY (+ _PASSWORD). En local usa build:unsigned
npm run build:unsigned # tauri build sin artefactos del updater (src-tauri/tauri.unsigned.conf.json)
npm run build:web      # vue-tsc --noEmit + vite build → dist/ (beforeBuildCommand de Tauri)
npm run preview        # vite preview de dist/
npm run typecheck      # vue-tsc de la app y de tsconfig.node.json (lo que corre CI)
npm run lint           # eslint . --max-warnings 0: falla con cualquier warning
npm run lint:fix       # ESLint con autofix
npm run format         # prettier --write . (no lo corras en todo el repo: reformatea archivos ajenos al cambio)
npm run format:check   # prettier --check . (CI no lo exige)
npm run test           # = vitest run
npm run test:renderer  # Vitest (happy-dom): componentes, stores y capa API
npm run test:watch     # Vitest en modo watch
npm run test:database  # cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri          # CLI de Tauri (p. ej. `npm run tauri info`); tauri-action lo usa para compilar

# Dentro de src-tauri/ (lo mismo que corre CI):
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

- `generate_context!()` exige que exista `dist/index.html` para compilar el crate: corre `npm run build:web` una vez (CI crea un relleno).
- Mientras `plugins.updater.pubkey` de `src-tauri/tauri.conf.json` tenga el marcador `REEMPLAZAR…`, `npm run build` y el workflow de release fallan a propósito. `build:unsigned` funciona siempre.
- Versión: la fuente única es `package.json` (`tauri.conf.json` usa `"version": "../package.json"`). Mantén `version` de `src-tauri/Cargo.toml` igual a mano y, tras cambiarla, actualiza `src-tauri/Cargo.lock` con `cargo update --workspace --offline` (dentro de `src-tauri/`) y súbelo en el mismo commit: CI usa `--locked` y falla con un lock desfasado. `npm version <x.y.z> --no-git-tag-version` sube a la vez `package.json` y `package-lock.json`.

**CI** (`.github/workflows/ci.yml`; push a `main`/`dev`, PRs a `main` y a mano):

- `web` (Ubuntu): `npm ci`, `lint`, `typecheck`, `test:renderer` y `build:web`.
- `rust` (ubuntu-22.04, macos-latest y windows-latest, Rust 1.99.1): `cargo fmt` (solo Linux), `clippy -D warnings` y `cargo test`, todo con `--locked`. El código bajo `#[cfg(target_os = …)]` (PDF por SO, `RunEvent` de macOS) solo se compila de verdad para los 3 SO en este job.
- `bundle` (solo PRs a `main` y `workflow_dispatch`): `npm run build:unsigned -- --debug` con `--bundles nsis` en Windows y `deb,appimage` en Ubuntu 22.04. Sube los instaladores como artefactos `cuentacobro-qa-Windows`/`cuentacobro-qa-Linux` (14 días).
- `build`: agregador (`needs: [web, rust, bundle]`, `if: always()`) que exige el ruleset de `main`. Acepta `bundle` = `skipped`.

El trabajo diario va en la rama `dev`; los PRs apuntan a `main`.

**Releases** (`.github/workflows/release.yml`, tags `v*`, repo `necro304/cuenta_de_cobro_escritorio`):

- `create-release`: comprueba que el tag coincida con `package.json`, que la pubkey no tenga el marcador y que existan los secretos de firma. Crea el release en borrador con `docs/releases/<tag>.md` como notas.
- `build`: tauri-action v1 compila macOS universal, Windows x64 y ARM64 y Linux x64; firma y fusiona `latest.json` (`retryAttempts: 3`).
- `publish`: verifica la versión de `latest.json` y sus claves por plataforma e instalador (`linux-x86_64-deb`, `windows-x86_64-nsis`, …) y le copia las notas del release (tauri-action v1 las deja vacías cuando recibe `releaseId`; sin notas el job falla). Si existe la variable `ELECTRON_BRIDGE_TAG`, re-sube los yml puente para los usuarios 1.x. Publica como _Latest_; un tag con guion sale como prerelease.
- El updater lee `https://github.com/necro304/cuenta_de_cobro_escritorio/releases/latest/download/latest.json`. Ningún release posterior de Electron puede quedar como _Latest_.
- Limitación conocida: las URLs de descarga de `latest.json` (tauri-action v1) apuntan a `api.github.com/repos/…/releases/assets/<id>`, que cuenta contra el límite de 60 peticiones por hora por IP sin autenticar.

## Arquitectura

### Backend Rust (`src-tauri/`)

Crate `cuenta-cobro` (lib `cuentacobro_lib`, edición 2024):

- `src/main.rs` — llama a `cuentacobro_lib::run()`; sin consola en Windows en release.
- `src/lib.rs` — `Database` (`Arc<Mutex<Option<Connection>>>` + `path`; `None` = cerrada), plugins (single-instance siempre primero; luego log, dialog, process, updater y window-state sin `VISIBLE` e ignorando `pdf-*`), `setup` (migración heredada → BD → ventana `main` → autoprueba), `generate_handler!` con los 11 comandos, `legacy_migration_notice` y `RunEvent` de macOS (todo el `match` bajo `cfg`).
- `src/db.rs` — `open_connection` (FK, `busy_timeout` 5 s, DQS off, `journal_mode=DELETE`), `SCHEMA_SQL`, `MIGRATIONS` y la fila de perfil `id = 1`.
- `src/generic.rs`, `src/sqlvalue.rs`, `src/commands.rs` — `db_query`/`db_get`/`db_run` con paridad de better-sqlite3 (enlace, tipos de las filas y textos de error), y los comandos de cuenta de cobro.
- `src/cuenta_de_cobro.rs`, `src/money.rs`, `src/types.rs` — módulo profundo de creación/edición: valida referencias y decimales, deriva total/estado, protege abonos y persiste cabecera + conceptos en una transacción (BigUint, centavos u64, `ryu-js`). `types.rs` ⇔ `src/types/cuentaDeCobro.ts` campo a campo. No duplicar estas reglas en el renderer.
- `src/system.rs` — `db_backup`, `db_restore` (en éxito llama a `app.restart()`), `export_pdf`, `export_client_report_pdf`, `print_ready`, nombres de archivo por defecto y `MSG_CANCELLED = "Operación cancelada."` (contrato con el renderer).
- `src/pdf/{mod,macos,windows,linux}.rs` — ventana oculta `pdf-<n>` y PDF nativo por SO (WKWebView, WebView2 `PrintToPdf`, WebKitGTK). Plan B si un SO falla: `PdfWindowMode::Offscreen`.
- `src/pdf/selftest.rs` — autoprueba `CUENTACOBRO_PDF_SELFTEST`; solo compila con `debug_assertions`.
- `src/legacy_migration.rs` — importación de la BD de Electron en el primer arranque (con 9 tests unitarios).
- `tests/*.rs` — `cuenta_de_cobro` (harness portado de la 1.x; la interfaz `open`/`save` es la superficie de test), `edge`, `money`, `generic` (incluye el paso de WAL a DELETE) y `system`.
- `tauri.conf.json` (ventana `main` con `create: false` y `zoomHotkeysEnabled`, CSP, bundles, updater), `tauri.unsigned.conf.json`, `capabilities/` (`default` para `main`; `pdf-export` para `pdf-*`, solo `core:default`) y `windows/installer-hooks.nsh` (retira la 1.x instalada por usuario en Windows).

Reglas:

- **Columnas nuevas**: sigue el patrón de `MIGRATIONS` en `db.rs` (`ALTER TABLE` cuyo error se ignora si la columna ya existe). `open_connection` fuerza `journal_mode=DELETE`: no usar WAL, porque el respaldo copia solo el archivo principal.
- **Comando nuevo**: registrarlo en `generate_handler!` de `lib.rs`, añadirlo a `BackendCommand` y su función en `src/api/backend.ts`, y testearlo en `src/api/backend.test.ts`. Los rechazos son `String`; los parámetros de primer nivel son de una palabra; los campos de structs anidados van en camelCase (`#[serde(rename_all = "camelCase")]`) y las filas de la BD con los nombres de columna tal cual. Los comandos propios no necesitan ACL; los permisos de plugins van en `capabilities/`.
- **Diálogos de archivo**: siempre desde Rust con la API de callback (`save_file`/`pick_file` + `tokio::sync::oneshot`), nunca `blocking_*` dentro de un comando `async`. El webview no tiene permisos `dialog:*` ni `fs:*`.
- **Ventanas `pdf-*`**: Rust abre `index.html#/print/{id}?pdf=1&template=…&signature=0|1` o `#/client-report/{id}?pdf=1` en una ventana oculta de 900×400 px (no subir el alto: WKWebView pagina al menos el alto del viewport y saldría una hoja en blanco). La vista llama a `signalPrintReady()` → comando `print_ready` y Rust imprime: Carta con márgenes de 15 mm para la cuenta (espera hasta 10 s) y A4 sin márgenes para el informe (15 s). window-state las ignora.
- **Autoprueba de PDF** (solo builds debug): `CUENTACOBRO_PDF_SELFTEST=invoice:<invoiceId>:<default|simple>:<0|1>` o `report:<clientId>`, y `CUENTACOBRO_PDF_SELFTEST_OUT=<ruta absoluta>`. Espera 3 s tras crear `main`, exporta sin diálogo, escribe `PDF_SELFTEST OK <ruta>` o `PDF_SELFTEST ERROR <msg>` en stderr y sale con 0 o 1.
- **Migración heredada**: si no existe `app_data_dir/database.sqlite`, copia la BD de Electron desde `config_dir/cuenta-cobro/database.sqlite` (luego `config_dir/CuentaCobro/…`) con `VACUUM INTO` desde una conexión de solo lectura o, si falla, con copia cruda de `-journal`/`-wal`. Valida, renombra de forma atómica y escribe `legacy-migration.json` (`source`, `method` = `vacuum_into`|`raw_file_copy`, `migratedAtUnix`, `appVersion`, `noticeAcknowledged`). Nunca toca el original. Si la migración falla: diálogo y `exit(1)` sin crear BD. Si la BD no abre: la ventana se crea con la conexión en `None`, los comandos fallan con `The database connection is not open` y Configuración → Restaurar sigue disponible.

|                                                    | macOS                                                                 | Windows                                                   | Linux                                                     |
| -------------------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------- | --------------------------------------------------------- |
| Datos (`database.sqlite`, `legacy-migration.json`) | `~/Library/Application Support/com.cuentacobro.app/`                  | `%APPDATA%\com.cuentacobro.app\`                          | `~/.local/share/com.cuentacobro.app/`                     |
| BD heredada (en orden)                             | `~/Library/Application Support/cuenta-cobro/`, luego `…/CuentaCobro/` | `%APPDATA%\cuenta-cobro\`, luego `%APPDATA%\CuentaCobro\` | `~/.config/cuenta-cobro/`, luego `~/.config/CuentaCobro/` |
| Log                                                | `~/Library/Logs/com.cuentacobro.app/CuentaCobro.log`                  | `%LOCALAPPDATA%\com.cuentacobro.app\logs\`                | `~/.local/share/com.cuentacobro.app/logs/`                |

En macOS, Finder muestra `com.cuentacobro.app` como si fuera una app (el identificador termina en `.app`; tauri-cli lo advierte en cada build). El identificador no se cambia.

### Renderer (`src/`)

- `src/api/backend.ts` es el **único** módulo que importa `@tauri-apps/api/core`. Exporta `dbQuery`, `dbGet`, `dbRun`, `cuentaDeCobro`, `backupDatabase`, `restoreDatabase`, `exportInvoicePdf`, `exportClientReportPdf`, `notifyPrintReady`, `legacyMigrationNotice` e `isDesktop`. Las acciones nunca rechazan: devuelven `ActionOutcome` (`done`/`cancelled`/`failed`) y `cancelled` no muestra toast. Fuera de Tauri (Vitest, navegador) el SQL rechaza con `BackendError(DESKTOP_ONLY_MESSAGE)` y las acciones devuelven `failed`.
- Las vistas y los stores usan `dbQuery<T>`/`dbGet<T>`/`dbRun` con el genérico explícito; `dbGet` devuelve `undefined` si no hay fila. `DbParam = string | number | null`: los booleanos se pasan como `flag ? 1 : 0`, porque Rust replica la semántica de better-sqlite3, incluidos sus textos de error.
- `InvoiceEditor.vue` usa `cuentaDeCobro.open/save` y no escribe en `invoices` ni `invoice_items` directamente.
- Interfaces de dominio en `src/types/index.ts` (`Profile`, `Client`, `Invoice`, `InvoiceItem`, `InvoicePayment`, `BankAccount`, `TemplateId`); los tipos de cuenta de cobro en `src/types/cuentaDeCobro.ts`. `Invoice.status` es `'draft' | 'paid' | 'partially_paid'`.
- Stores Pinia (setup stores) en `src/stores/`: `profile` (`load`, `saveDetails`, `saveSignature`, `savePreferences`), `clients` (`load`, `create`, `update`, `remove`), `bankAccounts` (`load`, `save`, `remove`, `setDefault`), `theme` (`useColorMode` con `storageKey: 'theme'`) y `updater`.
- Actualizaciones: solo en el frontend, con el store `updater` y `UpdateDialog.vue` (`@tauri-apps/plugin-updater` y `relaunch()` de `@tauri-apps/plugin-process`); Rust solo registra el plugin. `checkOnStartup()` y `legacyMigrationNotice()` (toast «Datos importados») solo los llama `MainLayout.vue` en `onMounted`.
- Router con hash history (`src/router/index.ts`): las vistas principales viven bajo `MainLayout` (sidebar); `/print/:id` (query `pdf=1`, `template`, `signature`) y `/client-report/:id` (query `pdf=1`) van **fuera** del layout. En modo `pdf=1` la vista llama a `signalPrintReady()` (`src/lib/printReady.ts`) una sola vez, también si la carga falla; no depende de `requestAnimationFrame`, que no corre en ventanas ocultas.
- Impresión/PDF: `InvoicePrint.vue` imprime con `window.print()` (`@media print`) y descarga con `exportInvoicePdf`. Las plantillas viven en `src/components/templates/` (`DefaultTemplate`, `SimpleTemplate`). Para agregar una: crear el componente, añadir su id a `TemplateId` y registrarla en el `Select` de InvoicePrint.
- `src/lib/numeroALetras.ts` convierte montos a letras en español (requisito de la cuenta de cobro).
- Componentes UI Shadcn/Vue en `src/components/ui/`; alias `@` → `src/`. Iconos de `@lucide/vue` (no `lucide-vue-next`, que está deprecado).
- Tailwind 4 CSS-first con `@tailwindcss/vite`: tema, `@theme inline`, `tw-animate-css` y `@custom-variant dark (&:is(.dark *))` en `src/assets/index.css`. **Sin** `tailwind.config.js` ni `postcss.config.js`; `components.json` con `tailwind.config: ""`. Las variables de color guardan canales HSL sin envolver porque Chart.js las lee con `getComputedStyle`.

## Convenciones

- Nunca usar `any` — usar tipos de `@/types` o genéricos; los dbQuery/dbGet siempre con genérico explícito.
- Errores al usuario con `toast({ variant: 'destructive' })`, nunca `console.error` en el renderer.
- Validación de formularios: función `validate()` que retorna `string | null` antes de guardar (ver `InvoiceEditor.vue`, `Clients.vue`).
- El perfil es un singleton: siempre `WHERE id = 1`.
- Nunca ejecutar la app con el `HOME` real en pruebas automáticas: el primer arranque importaría la BD real de la 1.x y consumiría el aviso. Antes de lanzarla, comprobar que no hay otra CuentaCobro 2.x abierta (`pgrep -x cuenta-cobro` vacío): la instancia única usa un socket global (`/tmp/com_cuentacobro_app_si.sock` en macOS) que el `HOME` falso no aísla, y la segunda instancia sale con 0 sin hacer nada.
- Docs y comentarios en español. Los comentarios de código no citan documentos fuera del repo; si hace falta una referencia, se cita `docs/adr/0001-migracion-a-tauri.md`.

## Agent skills

### Issue tracker

Los issues y specs se gestionan como archivos Markdown locales bajo `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Se usan las cinco etiquetas canónicas: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` y `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

El repositorio usa un esquema single-context. See `docs/agents/domain.md`.
