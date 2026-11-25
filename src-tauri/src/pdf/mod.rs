//! Exportación directa a PDF (vectorial, sin diálogo de impresión).
//!
//! Equivalente Tauri 2 de `webContents.printToPDF` de Electron:
//! 1. Se abre una ventana OCULTA que carga una ruta hash del frontend
//!    (`index.html#/print/:id?pdf=1...`).
//! 2. La vista avisa que terminó de pintar invocando el comando `print_ready`
//!    (definido en `system.rs`), que llama a [`PrintReadyRegistry::notify`] con
//!    el label del webview que invoca.
//! 3. Se imprime a archivo con la API nativa del motor web de cada plataforma
//!    (WKWebView/NSPrintOperation, WebView2 PrintToPdf, WebKitGTK PrintOperation).
//! 4. La ventana se destruye en todos los caminos (éxito, error, timeout).
//!
//! Nada aquí bloquea el hilo principal: `with_webview` solo encola un closure
//! en el event loop y el resultado vuelve por un `oneshot` que se espera con
//! timeout desde un comando `async`.
//!
//! Este módulo no conoce el contrato con el renderer (`{ success, message }`,
//! textos de Electron, diálogo de guardado): eso vive en `system.rs`.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use tauri::{
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tokio::sync::oneshot;

/// Autoprueba de exportación sin diálogo (solo compilaciones debug; ver `selftest.rs`).
#[cfg(debug_assertions)]
pub mod selftest;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
use self::windows as platform;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;

/// Prefijo de label de las ventanas ocultas (debe coincidir con la capability
/// `pdf-export` y con el filtro de tauri-plugin-window-state).
pub const PDF_WINDOW_PREFIX: &str = "pdf-";

/// Tiempo máximo que puede tardar el motor nativo en escribir el PDF.
const PRINT_TIMEOUT: Duration = Duration::from_secs(60);

/// Tamaño (px CSS) de la ventana oculta. El ALTO debe ser menor que el área imprimible más
/// pequeña (Carta con márgenes de 15 mm ≈ 942 px; A4 sin márgenes ≈ 1123 px): WKWebView pagina
/// al menos el alto del viewport, y con 1200 px cada cuenta de cobro salía con una 2.ª hoja en
/// blanco (verificado con las vistas reales en macOS; con 400 px sale 1 hoja). El ANCHO lo usa la
/// corrección de escala de WebKit (`webkit_print_init_script`).
const PDF_WINDOW_SIZE: (f64, f64) = (900.0, 400.0);

/// Tiempo máximo esperando el evento `Destroyed` de la ventana oculta.
const DESTROY_TIMEOUT: Duration = Duration::from_secs(3);

/// Geometría de página en milímetros (independiente de la plataforma).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfPage {
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_top_mm: f64,
    pub margin_right_mm: f64,
    pub margin_bottom_mm: f64,
    pub margin_left_mm: f64,
    /// Equivale a `printBackground: true` de Electron.
    pub print_background: bool,
}

impl PdfPage {
    /// Carta (8.5 × 11 in) con el mismo margen en los cuatro lados.
    pub const fn letter(margin_mm: f64) -> Self {
        Self::new(215.9, 279.4, margin_mm)
    }

    /// A4 (210 × 297 mm) con el mismo margen en los cuatro lados.
    pub const fn a4(margin_mm: f64) -> Self {
        Self::new(210.0, 297.0, margin_mm)
    }

    const fn new(width_mm: f64, height_mm: f64, margin_mm: f64) -> Self {
        Self {
            width_mm,
            height_mm,
            margin_top_mm: margin_mm,
            margin_right_mm: margin_mm,
            margin_bottom_mm: margin_mm,
            margin_left_mm: margin_mm,
            print_background: true,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn mm_to_pt(mm: f64) -> f64 {
    mm * 72.0 / 25.4
}

#[allow(dead_code)]
pub(crate) fn mm_to_in(mm: f64) -> f64 {
    mm / 25.4
}

/// Paridad de escala con Chromium/Electron en motores WebKit (macOS y Linux).
///
/// Chromium imprime a 96 px CSS por pulgada (1 px = 0.75 pt). WebKit, hasta el
/// commit a47d3db47b (2026-10-31, "Print Page Content at 96 px per inch"),
/// maqueta la impresión a 1.25 px por punto (1 px = 0.8 pt): todo sale 6.7 %
/// más grande y un bloque de `height: 297mm` ocupa más de una hoja A4.
///
/// Como conviven WebKits viejos y nuevos, no se fija el zoom a ciegas: `<html>`
/// pasa a ser contenedor (solo en impresión) y una container query compara su
/// ancho real de maquetación con el umbral entre ambos comportamientos; solo en
/// el viejo aplica `zoom: 0.9375` (= 0.75 / 0.8) a `<body>`.
/// (Las media queries NO sirven: en WKWebView `@media print and (width)` evalúa
/// el ancho de la ventana —880 px en la prueba—, no el de la página.)
/// En Chromium/WebView2 no se inyecta: ya maqueta a 96 px/in.
///
/// Con `print_background` añade además `print-color-adjust: exact` (propiedad
/// heredada) en `<html>`: en macOS < 13.3 no existe `shouldPrintBackgrounds` y
/// es la única forma de que WebKit imprima los fondos. Solo afecta a las
/// ventanas ocultas de PDF, no al botón «Imprimir» de la ventana principal.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn webkit_print_init_script(page: &PdfPage) -> String {
    let printable_pt = mm_to_pt(page.width_mm - page.margin_left_mm - page.margin_right_mm);
    let width_96dpi_px = printable_pt * 96.0 / 72.0; // ancho de maquetación correcto
    let width_legacy_px = printable_pt * 1.25; // ancho en WebKit antiguo
    let threshold_px = ((width_96dpi_px + width_legacy_px) / 2.0).floor();
    let backgrounds = if page.print_background {
        " html { -webkit-print-color-adjust: exact; print-color-adjust: exact; }"
    } else {
        ""
    };
    format!(
        r#"(() => {{
  const css = '@media print {{ html {{ container-type: inline-size; }}{backgrounds} @container (max-width: {threshold_px}px) {{ body {{ zoom: 0.9375; }} }} }}';
  // Hoja construida (CSSOM): la CSP no la bloquea, a diferencia de un <style>
  // cuando Tauri añade nonces/hashes a style-src.
  try {{
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(css);
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
    return;
  }} catch (_) {{
    // WebKit sin hojas construidas (< Safari 16.4 / WebKitGTK 2.40): <style>.
  }}
  const add = () => {{
    const style = document.createElement('style');
    style.dataset.pdfPrintFix = '';
    style.textContent = css;
    (document.head || document.documentElement).appendChild(style);
  }};
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', add, {{ once: true }});
  else add();
}})();"#
    )
}

/// Cómo se crea la ventana que renderiza el documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PdfWindowMode {
    /// Ventana nunca mostrada (`visible(false)`). Modo por defecto.
    #[default]
    Hidden,
    /// Plan B: ventana "visible" pero fuera de pantalla y sin barra de tareas.
    Offscreen,
}

/// Error de [`render_route_to_pdf`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    /// La vista no llamó a `print_ready` dentro del plazo (el llamador elige el
    /// texto: "…generando el PDF" o "…generando el informe").
    ReadyTimeout,
    /// Cualquier otro fallo, con un mensaje en español para el usuario.
    Failed(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadyTimeout => f.write_str("Tiempo de espera agotado generando el PDF"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for PdfError {}

fn failed(message: impl Into<String>) -> PdfError {
    PdfError::Failed(message.into())
}

type LoadResult = Result<(), String>;
type PendingSignals = HashMap<String, oneshot::Sender<LoadResult>>;

/// Estado gestionado (`app.manage`): resultados de carga pendientes,
/// por label de ventana. Equivale al `Map<webContentsId, resolve>` de Electron.
#[derive(Default)]
pub struct PrintReadyRegistry {
    pending: Mutex<PendingSignals>,
}

impl PrintReadyRegistry {
    fn insert(&self, label: &str, tx: oneshot::Sender<LoadResult>) {
        self.lock().insert(label.to_owned(), tx);
    }

    fn remove(&self, label: &str) -> Option<oneshot::Sender<LoadResult>> {
        self.lock().remove(label)
    }

    /// Lo llama el comando `print_ready` con el label del webview que invoca.
    /// Desde una ventana que no está exportando no tiene efecto.
    pub fn notify(&self, label: &str, result: LoadResult) {
        if let Some(tx) = self.remove(label) {
            let _ = tx.send(result);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PendingSignals> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Serializa las exportaciones: una sola ventana de PDF a la vez.
static EXPORT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Destruye la ventana oculta y limpia el registro al salir de cualquier camino.
struct HiddenWindowGuard<R: Runtime> {
    app: AppHandle<R>,
    label: String,
    window: Option<WebviewWindow<R>>,
}

impl<R: Runtime> Drop for HiddenWindowGuard<R> {
    fn drop(&mut self) {
        self.app.state::<PrintReadyRegistry>().remove(&self.label);
        if let Some(window) = self.window.take() {
            let _ = window.destroy();
        }
    }
}

/// Renderiza `route` (ruta hash del router, p. ej. `/print/12?pdf=1`) en una
/// ventana oculta y la guarda como PDF vectorial en `output`.
pub async fn render_route_to_pdf<R: Runtime>(
    app: &AppHandle<R>,
    route: &str,
    page: PdfPage,
    output: &Path,
    ready_timeout: Duration,
    mode: PdfWindowMode,
) -> Result<(), PdfError> {
    let _serial = EXPORT_LOCK.lock().await;

    let label = format!(
        "{PDF_WINDOW_PREFIX}{}",
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    );
    let (ready_tx, ready_rx) = oneshot::channel();
    app.state::<PrintReadyRegistry>().insert(&label, ready_tx);
    let mut guard = HiddenWindowGuard {
        app: app.clone(),
        label: label.clone(),
        window: None,
    };

    let url = WebviewUrl::App(PathBuf::from(format!("index.html#{route}")));
    let mut builder = WebviewWindowBuilder::new(app, &label, url)
        .title("CuentaCobro PDF")
        .inner_size(PDF_WINDOW_SIZE.0, PDF_WINDOW_SIZE.1)
        .skip_taskbar(true)
        .focused(false)
        .resizable(false);
    builder = match mode {
        PdfWindowMode::Hidden => builder.visible(false),
        PdfWindowMode::Offscreen => builder.visible(true).position(-20_000.0, -20_000.0),
    };
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        builder = builder.initialization_script(webkit_print_init_script(&page));
    }
    let window = builder
        .build()
        .map_err(|e| failed(format!("No se pudo crear la ventana de PDF: {e}")))?;

    let (destroyed_tx, destroyed_rx) = oneshot::channel::<()>();
    let destroyed_tx = Mutex::new(Some(destroyed_tx));
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            let mut slot = destroyed_tx.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(tx) = slot.take() {
                let _ = tx.send(());
            }
        }
    });
    guard.window = Some(window.clone());

    let result = print_when_ready(ready_rx, ready_timeout, output, &label, |target| {
        print_to_file(&window, page, target)
    })
    .await;

    // Limpieza determinista: se destruye la ventana y se ESPERA su evento
    // `Destroyed` antes de soltar EXPORT_LOCK. Crear la siguiente ventana de PDF
    // mientras la anterior aún se destruye dejó colgadas ~1 de cada 5
    // exportaciones consecutivas en macOS (medido con 15 exportaciones seguidas).
    if let Some(window) = guard.window.take() {
        let _ = window.destroy();
        let _ = tokio::time::timeout(DESTROY_TIMEOUT, destroyed_rx).await;
    }
    result
    // `guard` (sin ventana ya) limpia el registro; si el futuro se cancela antes
    // de llegar aquí, su Drop destruye la ventana igualmente.
}

async fn print_when_ready<Fut: std::future::Future<Output = Result<(), PdfError>>>(
    ready_rx: oneshot::Receiver<LoadResult>,
    ready_timeout: Duration,
    output: &Path,
    label: &str,
    print: impl FnOnce(PathBuf) -> Fut,
) -> Result<(), PdfError> {
    match tokio::time::timeout(ready_timeout, ready_rx).await {
        Ok(Ok(result)) => result.map_err(failed)?,
        Ok(Err(_)) => {
            return Err(failed(
                "La vista de impresión se cerró antes de estar lista",
            ));
        }
        Err(_) => return Err(PdfError::ReadyTimeout),
    }

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(failed(format!(
            "La carpeta destino no existe: {}",
            parent.display()
        )));
    }
    // El motor escribe en un temporal y solo un PDF completo reemplaza a `output`: si la
    // generación falla, el archivo que el usuario ya tenía con ese nombre queda intacto.
    let partial = partial_path(output, label);
    let _ = std::fs::remove_file(&partial);
    if let Err(error) = print(partial.clone()).await {
        let _ = std::fs::remove_file(&partial);
        return Err(error);
    }
    commit_pdf(&partial, output)
}

/// Imprime el webview a `target` con el motor nativo y espera a que termine.
async fn print_to_file<R: Runtime>(
    window: &WebviewWindow<R>,
    page: PdfPage,
    target: PathBuf,
) -> Result<(), PdfError> {
    let (done_tx, done_rx) = oneshot::channel::<Result<(), String>>();
    window
        .with_webview(move |webview| platform::print_to_pdf(webview, &target, page, done_tx))
        .map_err(|e| failed(format!("No se pudo acceder al webview: {e}")))?;

    match tokio::time::timeout(PRINT_TIMEOUT, done_rx).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(message))) => Err(failed(message)),
        Ok(Err(_)) => Err(failed("El motor de impresión terminó sin responder")),
        Err(_) => Err(failed("Tiempo de espera agotado escribiendo el PDF")),
    }
}

/// Temporal oculto donde escribe el motor: en la misma carpeta que `output` (así `rename` lo
/// reemplaza de forma atómica) y con extensión `.pdf`. `label` es el de la ventana `pdf-<n>`,
/// único en el proceso; las exportaciones van de una en una (`EXPORT_LOCK`).
fn partial_path(output: &Path, label: &str) -> PathBuf {
    output.with_file_name(format!(".cuentacobro-{label}.partial.pdf"))
}

/// Publica en `output` el PDF que el motor escribió en `partial`. Si `partial` no existe o está
/// vacío, `output` no se toca. En todos los casos `partial` deja de existir.
fn commit_pdf(partial: &Path, output: &Path) -> Result<(), PdfError> {
    if !std::fs::metadata(partial).is_ok_and(|meta| meta.len() > 0) {
        let _ = std::fs::remove_file(partial);
        return Err(failed("El motor de impresión no generó el archivo PDF"));
    }
    // Reemplaza el archivo existente: en Unix de forma atómica; en Windows, MoveFileExW con
    // MOVEFILE_REPLACE_EXISTING.
    std::fs::rename(partial, output).map_err(|e| {
        let _ = std::fs::remove_file(partial);
        failed(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANTERIOR: &[u8] = b"%PDF anterior";

    #[test]
    fn al_fallar_la_carga_no_imprime_ni_reemplaza_el_destino() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("informe.pdf");
        std::fs::write(&output, ANTERIOR).unwrap();
        let registry = PrintReadyRegistry::default();
        let (tx, rx) = oneshot::channel();
        registry.insert("pdf-1", tx);
        registry.notify("pdf-1", Err("Falló la consulta".into()));
        let result = tauri::async_runtime::block_on(print_when_ready(
            rx,
            Duration::from_secs(1),
            &output,
            "pdf-1",
            |_| async { panic!("Se intentó imprimir la vista fallida") },
        ));
        assert_eq!(result, Err(PdfError::Failed("Falló la consulta".into())));
        assert_eq!(std::fs::read(&output).unwrap(), ANTERIOR);
        assert!(!partial_path(&output, "pdf-1").exists());
    }

    #[test]
    fn sin_senal_de_la_ventana_correcta_vence_el_plazo_sin_publicar() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("cuenta.pdf");
        let registry = PrintReadyRegistry::default();
        let (tx, rx) = oneshot::channel();
        registry.insert("pdf-2", tx);
        registry.notify("pdf-1", Ok(()));
        let result = tauri::async_runtime::block_on(print_when_ready(
            rx,
            Duration::ZERO,
            &output,
            "pdf-2",
            |_| async { panic!("Se intentó imprimir sin señal de la ventana") },
        ));
        assert_eq!(result, Err(PdfError::ReadyTimeout));
        assert!(!output.exists());
        assert!(!partial_path(&output, "pdf-2").exists());
    }

    #[test]
    fn el_temporal_va_junto_al_destino_con_extension_pdf() {
        let output = Path::new("/descargas/cuenta_de_cobro_00007.pdf");
        let partial = partial_path(output, "pdf-3");
        assert_eq!(partial.parent(), output.parent());
        assert_eq!(partial.extension(), Some("pdf".as_ref()));
        assert_ne!(partial, output);
    }

    #[test]
    fn sin_pdf_nuevo_el_archivo_anterior_queda_intacto() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("cuenta_de_cobro_00007.pdf");
        std::fs::write(&output, ANTERIOR).unwrap();
        let partial = partial_path(&output, "pdf-1");

        // El motor no escribió nada.
        assert!(matches!(
            commit_pdf(&partial, &output),
            Err(PdfError::Failed(_))
        ));
        assert_eq!(std::fs::read(&output).unwrap(), ANTERIOR);

        // El motor dejó un archivo vacío.
        std::fs::write(&partial, b"").unwrap();
        assert!(matches!(
            commit_pdf(&partial, &output),
            Err(PdfError::Failed(_))
        ));
        assert_eq!(std::fs::read(&output).unwrap(), ANTERIOR);
        assert!(!partial.exists());
    }

    #[test]
    fn con_pdf_nuevo_reemplaza_el_anterior_y_borra_el_temporal() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("cuenta_de_cobro_00007.pdf");
        std::fs::write(&output, ANTERIOR).unwrap();
        let partial = partial_path(&output, "pdf-2");
        std::fs::write(&partial, b"%PDF nuevo").unwrap();

        commit_pdf(&partial, &output).unwrap();

        assert_eq!(std::fs::read(&output).unwrap(), b"%PDF nuevo");
        assert!(!partial.exists());
    }
}
