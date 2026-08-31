# Soporte de tema oscuro — Diseño

**Fecha:** 2026-06-06
**Estado:** Aprobado

## Objetivo

Agregar tema oscuro a CuentaCobro: por defecto sigue el tema del sistema, permite forzar claro u oscuro manualmente, y la elección persiste entre sesiones.

## Contexto existente

- `tailwind.config.js` ya tiene `darkMode: ['class']`.
- `src/assets/index.css` ya define todas las variables CSS del tema oscuro bajo `.dark` (variante oscura del estilo brutalist), incluidas las variantes `.dark &` de las utilities `shadow-brutal*`.
- `@vueuse/core` 14 está instalado y provee `useColorMode`.
- Hoy nada aplica la clase `dark`; el tema oscuro es código muerto.

## Decisión de enfoque

`useColorMode` de @vueuse/core. Alternativas descartadas:

- **Implementación manual** (matchMedia + localStorage): reinventa lo ya instalado.
- **`nativeTheme` de Electron vía IPC**: solo aportaría coherencia visual en 3 diálogos nativos (backup/restore/updater) a cambio de plumbing IPC y persistencia manual. Descartado por YAGNI; añadible después sin romper este diseño.

## Componentes

### 1. Composable `src/composables/useTheme.ts`

Envuelve `useColorMode({ emitAuto: true, storageKey: 'theme' })`.

- Estados: `'auto'` (default — sigue al sistema en vivo) | `'light'` | `'dark'`.
- Aplica la clase al `<html>`; Tailwind reacciona vía variant `dark:`.
- Persistencia automática en `localStorage['theme']`.
- Expone `{ theme, setTheme }` tipado (`ThemeMode = 'auto' | 'light' | 'dark'`).

### 2. Inicialización en `App.vue`

Llamar `useTheme()` en el setup de `App.vue` para que la clase se aplique desde el primer paint en cualquier ruta (incluida `/print/:id`, que vive fuera de MainLayout).

### 3. UI en `src/views/Settings.vue`

Nueva Card "Apariencia" entre "Acerca de" y "Respaldo y Restauración":

- Selector segmentado de 3 botones: ☀ Claro / ☾ Oscuro / Sistema.
- Iconos `Sun`, `Moon`, `Monitor` de `@lucide/vue`.
- Botón activo: `variant="default"`; inactivos: `variant="outline"` (patrón shadcn existente en la página).

### 4. Impresión siempre en claro (`src/views/InvoicePrint.vue`)

El documento es para papel/PDF; en oscuro sería ilegible. Al montar la vista se remueve la clase `dark` del `<html>` y al desmontar se restaura el estado del tema elegido. No cambia la preferencia persistida.

## Casos borde

- **Sin localStorage:** vueuse degrada silenciosamente; queda en `auto`.
- **FOUC:** no aplica — el composable corre en setup de `App.vue` antes del primer render.
- **Cambio de tema del sistema en vivo:** con `auto`, `useColorMode` escucha `prefers-color-scheme` y actualiza sin recargar.

## Verificación (manual; no hay framework de tests)

1. Default: primera ejecución sigue el tema del sistema.
2. Forzar Claro/Oscuro desde Configuración → cambia al instante.
3. Recargar/reabrir la app → mantiene la elección.
4. Con "Sistema" activo, cambiar el tema de macOS → la app reacciona en vivo.
5. Con tema oscuro activo, abrir `/print/:id` → se ve en claro; al volver, restaura oscuro.
