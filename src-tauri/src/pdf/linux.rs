//! Linux: `WebKitPrintOperation::print()` hacia la impresora virtual
//! "Print to File" de GTK 3, con `output-uri` y formato PDF (sin diálogo).
//!
//! - WebKitGTK busca la impresora por nombre EXACTO; el backend de archivo de
//!   GTK 3 la registra con `_("Print to File")`, es decir
//!   `g_dgettext("gtk30", "Print to File")`, que ya viene traducido según el
//!   idioma del sistema ("Imprimir a un archivo", etc.). Por eso no se puede
//!   escribir el nombre literal.
//! - La señal `failed` se emite antes que `finished`; gana la primera.
//! - Los fondos se controlan con `WebKitSettings:print-backgrounds`.

use std::{cell::RefCell, path::Path, rc::Rc};

use gtk::{PageOrientation, PaperSize, PrintSettings, Unit, glib};
use tauri::webview::PlatformWebview;
use tokio::sync::oneshot;
use webkit2gtk::{PrintOperation, PrintOperationExt, SettingsExt, WebViewExt};

use super::PdfPage;

type Done = oneshot::Sender<Result<(), String>>;

/// Se ejecuta en el hilo principal de GTK (closure de `with_webview`). No bloquea.
pub(super) fn print_to_pdf(webview: PlatformWebview, output: &Path, page: PdfPage, done: Done) {
    let view = webview.inner();

    if let Some(settings) = WebViewExt::settings(&view) {
        settings.set_print_backgrounds(page.print_background);
    }

    let uri = match glib::filename_to_uri(output, None) {
        Ok(uri) => uri,
        Err(error) => {
            let _ = done.send(Err(format!("Ruta de PDF inválida: {error}")));
            return;
        }
    };

    let paper = PaperSize::new_custom(
        "cuentacobro",
        "CuentaCobro",
        page.width_mm,
        page.height_mm,
        Unit::Mm,
    );

    let print_settings = PrintSettings::new();
    let file_printer = glib::dgettext(Some("gtk30"), "Print to File");
    print_settings.set_printer(&file_printer);
    print_settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT.as_str(), Some("pdf"));
    print_settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI.as_str(), Some(uri.as_str()));
    print_settings.set_paper_size(&paper);
    print_settings.set_orientation(PageOrientation::Portrait);
    print_settings.set_scale(100.0);

    let page_setup = gtk::PageSetup::new();
    page_setup.set_paper_size(&paper);
    page_setup.set_orientation(PageOrientation::Portrait);
    page_setup.set_top_margin(page.margin_top_mm, Unit::Mm);
    page_setup.set_right_margin(page.margin_right_mm, Unit::Mm);
    page_setup.set_bottom_margin(page.margin_bottom_mm, Unit::Mm);
    page_setup.set_left_margin(page.margin_left_mm, Unit::Mm);

    let operation = PrintOperation::new(&view);
    operation.set_print_settings(&print_settings);
    operation.set_page_setup(&page_setup);

    let done = Rc::new(RefCell::new(Some(done)));
    {
        let done = done.clone();
        operation.connect_failed(move |_, error| {
            if let Some(tx) = done.borrow_mut().take() {
                let _ = tx.send(Err(format!("WebKitGTK no pudo imprimir a PDF: {error}")));
            }
        });
    }
    {
        let done = done.clone();
        operation.connect_finished(move |_| {
            if let Some(tx) = done.borrow_mut().take() {
                let _ = tx.send(Ok(()));
            }
        });
    }

    // WebKit retiene la operación hasta emitir `finished`.
    operation.print();
}
