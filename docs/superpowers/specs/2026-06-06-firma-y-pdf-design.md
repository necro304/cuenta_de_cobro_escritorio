# Firma del emisor y descarga en PDF — Diseño

**Fecha:** 2026-06-06
**Estado:** Aprobado por el usuario

## Objetivo

Permitir que la cuenta de cobro incluya la firma del emisor (imagen subida o dibujada en pantalla) y se pueda descargar como PDF, tanto desde la vista previa como directamente desde la lista de cuentas. El uso de la firma se controla con tres modos: automática, sin firma, o preguntar al generar.

## Decisiones de enfoque

| Eje | Decisión | Alternativas descartadas |
| --- | --- | --- |
| Generación de PDF | `webContents.printToPDF()` de Electron sobre una ventana oculta que renderiza la vista de impresión existente | jsPDF/pdfmake (duplicaría las plantillas); html2canvas (PDF rasterizado) |
| Almacenamiento de la firma | Data URL base64 en columna `signature` de `profile` — viaja con el backup/restore actual (copia del `.sqlite`) | Archivo en `userData` (el backup no lo incluiría; requeriría IPC extra) |
| Canvas de dibujo | Librería `signature_pad` (trazo suavizado con curvas Bézier), exporta PNG transparente | Canvas nativo con pointer events |

## 1. Modelo de datos

Migraciones en `electron/database.ts` siguiendo el patrón existente (`ALTER TABLE` en try/catch que ignora "columna ya existe"):

- `profile.signature TEXT` — data URL (`data:image/png;base64,...`) de la firma.
- `profile.signature_mode TEXT DEFAULT 'auto'` — valores: `'auto' | 'none' | 'ask'`. Default `auto`: quien sube una firma normalmente quiere usarla.
- `profile.default_template TEXT DEFAULT 'default'` — valores: `'default' | 'simple'`.

En `src/types/index.ts`:

- Exportar `type SignatureMode = 'auto' | 'none' | 'ask'` y `type TemplateId = 'default' | 'simple'`.
- `Profile`: agregar `signature?: string | null`, `signature_mode?: SignatureMode`, `default_template?: TemplateId`; **eliminar** `signature_path` (columna legacy nunca usada; permanece en la DB pero sale del tipo).

## 2. Perfil — gestión de firma (`src/views/Profile.vue`)

Card nueva **"Firma"** con:

- **Vista previa** de la firma actual sobre fondo claro, o estado vacío ("No has configurado una firma").
- **Subir imagen**: `<input type="file" accept="image/png,image/jpeg">` → `FileReader.readAsDataURL`. Validación tipo `validate()`: solo PNG/JPG y máximo 1 MB; error con `toast({ variant: 'destructive' })`.
- **Dibujar firma**: Dialog con canvas manejado por `signature_pad` (componente nuevo `src/components/SignaturePadDialog.vue`). Botones: Limpiar, Cancelar, Guardar. Exporta `toDataURL('image/png')` con fondo transparente. Guardar deshabilitado si el pad está vacío (`isEmpty()`).
- **Eliminar firma**: pone `signature = NULL` (con confirmación).
- Select **"Uso de la firma"**: Automática / Sin firma / Preguntar al generar (`signature_mode`).
- Select **"Plantilla predeterminada"**: Por defecto / Simple (`default_template`).

Persistencia: los tres campos se guardan en el `UPDATE profile ... WHERE id = 1` existente del botón "Guardar Perfil"; subir/dibujar/eliminar la firma persiste inmediatamente (UPDATE puntual de `signature`) para no perder la imagen si no se pulsa Guardar.

Regla global: **si no hay firma guardada, los documentos salen sin firma sin importar el modo** y no se pregunta nada.

## 3. Plantillas (`src/components/templates/`)

Ambas plantillas reciben una prop nueva `signature: string | null` (null = no incluir):

- `DefaultTemplate.vue`: la imagen va centrada **sobre** `.firma-line` (la línea de firma), altura máx ~80 px.
- `SimpleTemplate.vue`: la imagen va entre "Cordialmente," y el nombre, reemplazando los `<br>` de espacio cuando hay firma; usa la clase `.signature-image` ya existente.

La imagen se renderiza con `<img :src="signature">` (data URL, no requiere protocolo file://).

## 4. Vista previa (`src/views/InvoicePrint.vue`)

- La plantilla seleccionada se inicializa con `profile.default_template`.
- Select nuevo **"Firma: Con firma / Sin firma"**, visible solo si `profile.signature` existe. Valor inicial según `signature_mode`: `auto` → Con firma; `none` y `ask` → Sin firma (en modo preguntar la vista previa se muestra sin firma hasta el momento de imprimir/descargar).
- Botón nuevo **"Descargar PDF"** junto a "Imprimir".
- **Modo `ask`**: al pulsar Imprimir o Descargar PDF se abre un Dialog "¿Incluir tu firma en este documento?" con botones "Con firma" / "Sin firma"; la elección actualiza el select y continúa la acción (para imprimir, `await nextTick()` antes de `window.print()`). Si el usuario ya cambió el select manualmente en esta vista, no se vuelve a preguntar.
- **Modo PDF embebido**: si la ruta trae `?pdf=1&template=X&signature=0|1`, la vista oculta los controles, fuerza plantilla y firma según los query params y, al terminar `loadData()`, llama `window.electronAPI.notifyPrintReady()`.

## 5. Generación de PDF (IPC)

Nuevo en `electron/main.ts` — handler `export-pdf`:

```
exportPdf({ invoiceId, invoiceNumber, template, includeSignature })
  → Promise<{ success: boolean; message: string }>
```

1. `dialog.showSaveDialog` con nombre sugerido `cuenta_de_cobro_<número con padding 5>.pdf` (default en Descargas). Si se cancela → `{ success: false, message: 'Operación cancelada.' }` (el renderer no muestra error en ese caso).
2. Crea `BrowserWindow` oculta (`show: false`, misma preload) y carga la app con hash `#/print/<invoiceId>?pdf=1&template=<template>&signature=<0|1>` (dev: `VITE_DEV_SERVER_URL`; prod: `loadFile` con hash).
3. Espera la señal `print-ready` de esa ventana (one-shot, filtrada por `webContents.id`), con **timeout de 10 s** → error si expira.
4. `printToPDF({ printBackground: true, pageSize: 'Letter', margins: 1.5 cm por lado })` — mismos márgenes que la impresión (`@page { margin: 1.5cm }`) para que PDF e impresión salgan idénticos. Escribe el archivo con `fs.writeFileSync` y cierra la ventana oculta (también en caso de error — try/finally).
5. Retorna éxito con la ruta en `message`.

Preload (`electron/preload.ts`) y tipos (`src/electron-api.d.ts`):

- `exportPdf(opts)` → `ipcRenderer.invoke('export-pdf', opts)`.
- `notifyPrintReady()` → `ipcRenderer.send('print-ready')`.

## 6. Lista de cuentas (`src/views/Invoices.vue`)

- Botón nuevo por fila (icono `Download` de `@lucide/vue`): descarga rápida del PDF usando `default_template` y `signature_mode` del perfil.
- Si `signature_mode === 'ask'` y hay firma → mismo Dialog "¿Incluir tu firma?" antes de invocar `exportPdf`.
- Resultado: toast de éxito mostrando la ruta de guardado, o `toast({ variant: 'destructive' })` si falla. Cancelación del diálogo de guardado: sin toast.

## Manejo de errores

- Imagen inválida (tipo/tamaño) → toast destructive, no se guarda.
- `export-pdf` con fallo (timeout, escritura, render) → `{ success: false }` + toast destructive.
- Ventana oculta siempre destruida (try/finally) para no filtrar procesos.

## Dependencias nuevas

- `signature_pad` (runtime, sin dependencias propias). No requiere rebuild nativo.

## Verificación

No hay tests configurados en el proyecto. Verificación:

1. `npx vue-tsc --noEmit` y `npm run lint` sin errores.
2. Manual: subir firma / dibujar firma / eliminar firma; los tres modos en imprimir y en descargar (vista previa y lista); plantilla predeterminada aplicada; PDF idéntico a la vista previa en ambas plantillas; backup/restore conserva la firma.

## Fuera de alcance

- Firmas por cliente o por cuenta de cobro individual.
- Firma digital criptográfica (esto es una imagen de firma manuscrita).
- Eliminar la columna legacy `signature_path` de la DB (SQLite no soporta DROP COLUMN sin recrear la tabla; solo sale del tipo).
