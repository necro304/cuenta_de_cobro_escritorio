//! Autoprueba de la exportación a PDF **sin diálogo** (solo compilaciones debug).
//!
//! Permite verificar el motor nativo con las vistas Vue reales y la BD real sin
//! intervención humana (el diálogo «Guardar como» es lo único que no cubre).
//!
//! Variables de entorno:
//! - `CUENTACOBRO_PDF_SELFTEST`: `invoice:<invoiceId>:<default|simple>:<0|1>` o `report:<clientId>`.
//! - `CUENTACOBRO_PDF_SELFTEST_OUT`: ruta absoluta del PDF a escribir.
//!
//! Si la primera no está definida no hace nada. Si está definida, espera 3 s a que
//! cargue la ventana principal, exporta, escribe `PDF_SELFTEST OK <ruta>` o
//! `PDF_SELFTEST ERROR <mensaje>` en stderr y termina el proceso con 0 o 1.

use std::{path::PathBuf, time::Duration};

use tauri::AppHandle;

use super::{PdfError, PdfPage, PdfWindowMode, render_route_to_pdf};
use crate::system::{
    ExportPdfOptions, INVOICE_MARGIN_MM, INVOICE_READY_TIMEOUT, REPORT_READY_TIMEOUT,
    client_report_route, invoice_print_route,
};

const ENV_SPEC: &str = "CUENTACOBRO_PDF_SELFTEST";
const ENV_OUT: &str = "CUENTACOBRO_PDF_SELFTEST_OUT";
const STARTUP_DELAY: Duration = Duration::from_secs(3);

/// Ruta hash, página y plazo de `print_ready` según la especificación de la variable.
fn plan(spec: &str) -> Result<(String, PdfPage, Duration), String> {
    let parts: Vec<&str> = spec.split(':').collect();
    let number = |s: &str| {
        s.parse::<f64>()
            .map_err(|e| format!("número inválido {s:?}: {e}"))
    };
    match parts.as_slice() {
        ["invoice", id, template, signature] => {
            let opts = ExportPdfOptions {
                invoice_id: number(id)?,
                invoice_number: 0.0,
                template: (*template).to_string(),
                include_signature: *signature == "1",
            };
            Ok((
                invoice_print_route(&opts),
                PdfPage::letter(INVOICE_MARGIN_MM),
                INVOICE_READY_TIMEOUT,
            ))
        }
        ["report", id] => Ok((
            client_report_route(number(id)?),
            PdfPage::a4(0.0),
            REPORT_READY_TIMEOUT,
        )),
        _ => Err(format!(
            "{ENV_SPEC} debe ser invoice:<id>:<default|simple>:<0|1> o report:<id>; recibido {spec:?}"
        )),
    }
}

/// Lanza la autoprueba si `CUENTACOBRO_PDF_SELFTEST` está definida. Llamar desde `setup`
/// después de crear la ventana principal.
pub fn maybe_run(app: &AppHandle) {
    let Ok(spec) = std::env::var(ENV_SPEC) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        let result = match (plan(&spec), std::env::var(ENV_OUT)) {
            (Err(e), _) => Err(e),
            (_, Err(_)) => Err(format!("falta {ENV_OUT} (ruta absoluta del PDF)")),
            (Ok((route, page, ready_timeout)), Ok(out)) => {
                let out = PathBuf::from(out);
                render_route_to_pdf(
                    &app,
                    &route,
                    page,
                    &out,
                    ready_timeout,
                    PdfWindowMode::Hidden,
                )
                .await
                .map(|()| out)
                .map_err(|e| match e {
                    PdfError::ReadyTimeout => "timeout esperando print_ready".to_string(),
                    PdfError::Failed(message) => message,
                })
            }
        };
        match result {
            Ok(out) => {
                eprintln!("PDF_SELFTEST OK {}", out.display());
                app.exit(0);
            }
            Err(message) => {
                eprintln!("PDF_SELFTEST ERROR {message}");
                app.exit(1);
            }
        }
    });
}
