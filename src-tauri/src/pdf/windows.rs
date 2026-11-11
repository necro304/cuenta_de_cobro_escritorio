//! Windows: `ICoreWebView2_7::PrintToPdf` + `ICoreWebView2PrintSettings`.
//!
//! - Requiere WebView2 Runtime ≥ 1.0.1020 (PrintToPdf) — cualquier Windows 10/11
//!   con Evergreen actualizado lo cumple.
//! - El callback llega en el hilo de UI cuando el archivo ya está escrito.
//! - Solo puede haber un PrintToPdf en curso por webview (el módulo serializa).

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use tauri::webview::PlatformWebview;
use tokio::sync::oneshot;
use webview2_com::{
    Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT, ICoreWebView2_7, ICoreWebView2Environment6,
    },
    PrintToPdfCompletedHandler,
};
use windows::core::{HSTRING, Interface};

use super::{PdfPage, mm_to_in};

type Done = oneshot::Sender<Result<(), String>>;
type SharedDone = Arc<Mutex<Option<Done>>>;

fn finish(done: &SharedDone, result: Result<(), String>) {
    if let Some(tx) = done.lock().unwrap_or_else(|p| p.into_inner()).take() {
        let _ = tx.send(result);
    }
}

/// Se ejecuta en el hilo de UI (closure de `with_webview`). No bloquea.
pub(super) fn print_to_pdf(webview: PlatformWebview, output: &Path, page: PdfPage, done: Done) {
    let done: SharedDone = Arc::new(Mutex::new(Some(done)));
    if let Err(error) = start(&webview, output, page, done.clone()) {
        finish(&done, Err(format!("WebView2 PrintToPdf: {error}")));
    }
}

fn start(
    webview: &PlatformWebview,
    output: &Path,
    page: PdfPage,
    done: SharedDone,
) -> windows::core::Result<()> {
    // SAFETY: llamadas COM sobre objetos vivos de esta ventana, en su hilo de UI
    // (closure de `with_webview`). Edición 2024: el cuerpo de una `unsafe fn`
    // ya no es contexto unsafe, por eso el bloque explícito.
    unsafe {
        let controller = webview.controller();
        // La ventana (HWND) sigue oculta; esto solo evita que WebView2 suspenda el
        // renderizado de un webview no visible mientras imprime.
        controller.SetIsVisible(true)?;

        let core = controller.CoreWebView2()?;
        let core7: ICoreWebView2_7 = core.cast()?;
        let environment6: ICoreWebView2Environment6 = webview.environment().cast()?;

        let settings = environment6.CreatePrintSettings()?;
        settings.SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT)?;
        settings.SetScaleFactor(1.0)?;
        settings.SetPageWidth(mm_to_in(page.width_mm))?;
        settings.SetPageHeight(mm_to_in(page.height_mm))?;
        settings.SetMarginTop(mm_to_in(page.margin_top_mm))?;
        settings.SetMarginRight(mm_to_in(page.margin_right_mm))?;
        settings.SetMarginBottom(mm_to_in(page.margin_bottom_mm))?;
        settings.SetMarginLeft(mm_to_in(page.margin_left_mm))?;
        settings.SetShouldPrintBackgrounds(page.print_background)?;
        settings.SetShouldPrintHeaderAndFooter(false)?;
        settings.SetShouldPrintSelectionOnly(false)?;

        let handler =
            PrintToPdfCompletedHandler::create(Box::new(move |error_code, is_successful| {
                let result = match (error_code, is_successful) {
                    (Ok(()), true) => Ok(()),
                    (Ok(()), false) => Err("WebView2 no pudo generar el PDF".to_string()),
                    (Err(error), _) => Err(format!("WebView2 PrintToPdf: {error}")),
                };
                finish(&done, result);
                Ok(())
            }));

        let path = HSTRING::from(output.as_os_str());
        core7.PrintToPdf(&path, &settings, &handler)
    }
}
