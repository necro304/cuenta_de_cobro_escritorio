//! db-backup, db-restore, export-pdf, export-client-report-pdf, print-ready.
use crate::{
    Database,
    money::js_number_to_string,
    pdf::{PdfError, PdfPage, PdfWindowMode, PrintReadyRegistry, render_route_to_pdf},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

pub const MSG_NO_WINDOW: &str = "No window found";
pub const MSG_CANCELLED: &str = "Operación cancelada.";
pub const MSG_BACKUP_OK: &str = "Respaldo guardado exitosamente.";
/// Electron lo devolvía, pero el proceso se reiniciaba antes de entregarlo: nunca llega al renderer.
#[allow(dead_code)]
pub const MSG_RESTORE_OK: &str = "Restauración completa. Reiniciando...";
pub const MSG_PDF_TIMEOUT: &str = "Tiempo de espera agotado generando el PDF";
pub const MSG_REPORT_TIMEOUT: &str = "Tiempo de espera agotado generando el informe";
/// El callback del diálogo nunca llegó (el plugin descartó el canal). No debería ocurrir.
const MSG_DIALOG_CLOSED: &str = "El diálogo se cerró inesperadamente";

#[derive(Debug, Serialize)]
pub struct OperationResult {
    pub success: bool,
    pub message: String,
}
fn ok(message: impl Into<String>) -> OperationResult {
    OperationResult {
        success: true,
        message: message.into(),
    }
}
fn fail(message: impl Into<String>) -> OperationResult {
    OperationResult {
        success: false,
        message: message.into(),
    }
}

#[tauri::command]
pub async fn db_backup(app: AppHandle, db: State<'_, Database>) -> Result<OperationResult, String> {
    let Some(main) = app.get_webview_window("main") else {
        return Ok(fail(MSG_NO_WINDOW));
    };
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Respaldar Base de Datos")
        .set_file_name("respaldo_cuentas_cobro.sqlite")
        .add_filter("SQLite Database", &["sqlite", "db"])
        .set_parent(&main);
    if let Ok(downloads) = app.path().download_dir() {
        dialog = dialog.set_directory(downloads);
    }
    // Sin `blocking_*`: el resultado llega por callback y no ocupa un hilo del runtime.
    let (tx, rx) = oneshot::channel();
    dialog.save_file(move |picked| {
        let _ = tx.send(picked);
    });
    let Some(target) = rx.await.map_err(|_| MSG_DIALOG_CLOSED.to_string())? else {
        return Ok(fail(MSG_CANCELLED));
    };
    let target = target.into_path().map_err(|e| e.to_string())?;
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || match crate::backup::save(&db, &target) {
        Ok(()) => ok(MSG_BACKUP_OK),
        Err(message) => fail(message),
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_restore(
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<OperationResult, String> {
    let Some(main) = app.get_webview_window("main") else {
        return Ok(fail(MSG_NO_WINDOW));
    };
    let (tx, rx) = oneshot::channel();
    app.dialog()
        .file()
        .set_title("Restaurar Base de Datos")
        .add_filter("SQLite Database", &["sqlite", "db"])
        .set_parent(&main)
        .pick_file(move |picked| {
            let _ = tx.send(picked);
        });
    let picked = rx.await.map_err(|_| MSG_DIALOG_CLOSED.to_string())?;
    let Some(source) = picked else {
        return Ok(fail(MSG_CANCELLED));
    };
    let source = source.into_path().map_err(|e| e.to_string())?;
    let db = db.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || crate::backup::restore(&db, &source))
        .await
        .map_err(|e| e.to_string())?;
    match result {
        Ok(()) => app.restart(),
        Err(message) => Ok(fail(message)),
    }
}

// ---------- PDF ----------
// Contrato con el renderer idéntico al de Electron ({ success, message } y los mismos
// textos). El motor (ventana oculta + impresión nativa) vive en `crate::pdf`.

/// Margen de la cuenta de cobro: igual que `@page { margin: 1.5cm }` de InvoicePrint.vue.
pub(crate) const INVOICE_MARGIN_MM: f64 = 15.0;
pub(crate) const INVOICE_READY_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const REPORT_READY_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPdfOptions {
    pub invoice_id: f64,
    pub invoice_number: f64,
    pub template: String,
    pub include_signature: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportClientReportOptions {
    pub client_id: f64,
    pub client_name: String,
}

pub fn invoice_pdf_default_name(invoice_number: f64) -> String {
    format!(
        "cuenta_de_cobro_{:0>5}.pdf",
        js_number_to_string(invoice_number)
    )
}

pub fn client_report_default_name(client_id: f64, client_name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let stripped: String = client_name
        .nfd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .collect();
    let mut out = String::new();
    let mut in_run = false;
    for c in stripped.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }
    let safe: String = out.trim_matches('_').chars().take(60).collect();
    if safe.is_empty() {
        format!("informe_cliente_{}.pdf", js_number_to_string(client_id))
    } else {
        format!("informe_{safe}.pdf")
    }
}

/// Hash de la vista de impresión de una cuenta (igual que el template literal de Electron).
pub fn invoice_print_route(opts: &ExportPdfOptions) -> String {
    format!(
        "/print/{}?pdf=1&template={}&signature={}",
        js_number_to_string(opts.invoice_id),
        opts.template,
        if opts.include_signature { 1 } else { 0 }
    )
}

/// Hash de la vista del informe del cliente.
pub fn client_report_route(client_id: f64) -> String {
    format!("/client-report/{}?pdf=1", js_number_to_string(client_id))
}

/// Diálogo nativo "Guardar como" (hoja de la ventana principal en macOS) en Descargas.
/// No bloquea ningún hilo: el resultado llega por callback. `None` = cancelado.
async fn ask_pdf_path(
    app: &AppHandle,
    parent: &WebviewWindow,
    title: &str,
    default_name: String,
) -> Result<Option<PathBuf>, String> {
    let mut dialog = app
        .dialog()
        .file()
        .set_title(title)
        .set_file_name(default_name)
        .add_filter("PDF", &["pdf"])
        .set_parent(parent);
    if let Ok(downloads) = app.path().download_dir() {
        dialog = dialog.set_directory(downloads);
    }
    let (tx, rx) = oneshot::channel();
    dialog.save_file(move |picked| {
        let _ = tx.send(picked);
    });
    let Some(picked) = rx
        .await
        .map_err(|_| "El diálogo de guardado se cerró inesperadamente".to_string())?
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    Ok(Some(pdf_target_path(path)))
}

/// Ruta donde se guarda el PDF a partir de la que devolvió el diálogo. GTK no añade la extensión
/// si el usuario la borra: se añade `.pdf` solo si ese archivo no existe, porque el diálogo
/// confirmó la sobrescritura de la ruta escrita, no la de `<nombre>.pdf`. En macOS y Windows el
/// diálogo ya devuelve la ruta con extensión y no cambia nada.
pub fn pdf_target_path(picked: PathBuf) -> PathBuf {
    if picked.extension().is_some() {
        return picked;
    }
    let with_pdf = picked.with_extension("pdf");
    match std::fs::symlink_metadata(&with_pdf) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => with_pdf,
        _ => picked,
    }
}

async fn export_view(
    app: &AppHandle,
    title: &str,
    default_name: String,
    route: String,
    page: PdfPage,
    ready_timeout: Duration,
    timeout_message: &str,
) -> Result<OperationResult, String> {
    let Some(main) = app.get_webview_window("main") else {
        return Ok(fail(MSG_NO_WINDOW));
    };
    let Some(target) = ask_pdf_path(app, &main, title, default_name).await? else {
        return Ok(fail(MSG_CANCELLED));
    };
    Ok(
        match render_route_to_pdf(
            app,
            &route,
            page,
            &target,
            ready_timeout,
            PdfWindowMode::Hidden,
        )
        .await
        {
            Ok(()) => ok(target.to_string_lossy()),
            Err(PdfError::ReadyTimeout) => fail(timeout_message),
            Err(PdfError::Failed(message)) => fail(message),
        },
    )
}

#[tauri::command]
pub async fn export_pdf(app: AppHandle, opts: ExportPdfOptions) -> Result<OperationResult, String> {
    export_view(
        &app,
        "Descargar PDF",
        invoice_pdf_default_name(opts.invoice_number),
        invoice_print_route(&opts),
        PdfPage::letter(INVOICE_MARGIN_MM),
        INVOICE_READY_TIMEOUT,
        MSG_PDF_TIMEOUT,
    )
    .await
}

#[tauri::command]
pub async fn export_client_report_pdf(
    app: AppHandle,
    opts: ExportClientReportOptions,
) -> Result<OperationResult, String> {
    export_view(
        &app,
        "Descargar informe del cliente",
        client_report_default_name(opts.client_id, &opts.client_name),
        client_report_route(opts.client_id),
        PdfPage::a4(0.0),
        REPORT_READY_TIMEOUT,
        MSG_REPORT_TIMEOUT,
    )
    .await
}

/// La vista en modo `?pdf=1` comunica que terminó de pintar o que falló la carga.
#[tauri::command]
pub fn print_ready(
    webview: tauri::Webview,
    registry: State<'_, PrintReadyRegistry>,
    error: Option<String>,
) {
    registry.notify(webview.label(), error.map_or(Ok(()), Err));
}
