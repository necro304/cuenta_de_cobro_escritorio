//! macOS: `WKWebView.printOperationWithPrintInfo:` + `NSPrintSaveJob`.
//!
//! Verificado en el prototipo (macOS 27, Xcode 27.2, Tauri 2.12.5, wry 0.57.2):
//! - Funciona con la ventana OCULTA (`visible(false)`): no aparece ninguna hoja,
//!   panel ni diálogo (`showsPrintPanel = NO`, `showsProgressPanel = NO`).
//! - Hay que usar `runOperationModalForWindow:delegate:didRunSelector:contextInfo:`
//!   (asíncrono). `runOperation()` (síncrono) bloquea el hilo principal y en la
//!   prueba escribió un archivo de 1.2 GB sin terminar nunca: NO usarlo.
//! - Se fija el frame de `operation.view` al tamaño del papel (recomendación
//!   de los foros de Apple; sin él AppKit puede fallar o paginar mal).
//! - `WKPreferences.shouldPrintBackgrounds` (macOS 13.3+) equivale a
//!   `printBackground` de Electron y surte efecto aunque se cambie después de
//!   crear el webview.
//! - WebKit respeta `@page { margin }` del CSS; aun así se pasan los mismos
//!   márgenes en NSPrintInfo (igual que Electron con `margins`).
//! - La escala (WebKit antiguo = 0.8 pt/px) se corrige en `mod.rs`
//!   (`webkit_print_init_script`).

use std::{cell::OnceCell, ffi::c_void, path::Path};

use objc2::MainThreadMarker;
use objc2::{
    MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool, NSObject, NSObjectProtocol},
    sel,
};
use objc2_app_kit::{
    NSPaperOrientation, NSPrintInfo, NSPrintJobSavingURL, NSPrintOperation, NSPrintSaveJob,
    NSPrintingPaginationMode,
};
use objc2_foundation::{NSCopying, NSPoint, NSRect, NSSize, NSString, NSURL};
use objc2_web_kit::WKWebView;
use tauri::webview::PlatformWebview;
use tokio::sync::oneshot;

use super::{PdfPage, mm_to_pt};

type Done = oneshot::Sender<Result<(), String>>;

/// Lo que viaja por `contextInfo` hasta el callback de AppKit.
struct PrintJob {
    done: Done,
    /// Mantiene viva la operación hasta que AppKit avisa que terminó.
    _operation: Retained<NSPrintOperation>,
}

define_class!(
    // SAFETY: NSObject no tiene requisitos de subclase; no implementamos Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "CuentaCobroPdfPrintDelegate"]
    struct PrintDelegate;

    impl PrintDelegate {
        /// `- (void)printOperationDidRun:(NSPrintOperation *)op success:(BOOL)ok contextInfo:(void *)ctx`
        #[unsafe(method(printOperationDidRun:success:contextInfo:))]
        fn print_operation_did_run(
            &self,
            _operation: &NSPrintOperation,
            success: Bool,
            context: *mut c_void,
        ) {
            if context.is_null() {
                return;
            }
            // SAFETY: `context` es el Box<PrintJob> que creamos abajo; AppKit lo
            // devuelve exactamente una vez.
            let job = unsafe { Box::from_raw(context.cast::<PrintJob>()) };
            let result = if success.as_bool() {
                Ok(())
            } else {
                Err("macOS no pudo completar la impresión a PDF".to_string())
            };
            let _ = job.done.send(result);
        }
    }
);

impl PrintDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: `init` de NSObject.
        unsafe { msg_send![super(this), init] }
    }
}

thread_local! {
    /// Un único delegate por proceso (solo se usa en el hilo principal).
    static DELEGATE: OnceCell<Retained<PrintDelegate>> = const { OnceCell::new() };
}

/// Se ejecuta en el hilo principal (closure de `with_webview`). No bloquea:
/// arranca la operación y el resultado llega por `done` desde el delegate.
pub(super) fn print_to_pdf(webview: PlatformWebview, output: &Path, page: PdfPage, done: Done) {
    let Some(mtm) = MainThreadMarker::new() else {
        let _ = done.send(Err(
            "print_to_pdf debe ejecutarse en el hilo principal".into()
        ));
        return;
    };

    // SAFETY: Tauri garantiza que `inner()` es el WKWebView vivo de esta ventana
    // durante el closure de `with_webview`.
    let wk: &WKWebView = unsafe { &*webview.inner().cast() };
    let Some(window) = wk.window() else {
        let _ = done.send(Err("El webview de PDF no tiene ventana".into()));
        return;
    };

    let paper = NSSize::new(mm_to_pt(page.width_mm), mm_to_pt(page.height_mm));
    let print_info: Retained<NSPrintInfo> = NSPrintInfo::sharedPrintInfo().copy();
    print_info.setPaperSize(paper);
    print_info.setOrientation(NSPaperOrientation::Portrait);
    print_info.setTopMargin(mm_to_pt(page.margin_top_mm));
    print_info.setRightMargin(mm_to_pt(page.margin_right_mm));
    print_info.setBottomMargin(mm_to_pt(page.margin_bottom_mm));
    print_info.setLeftMargin(mm_to_pt(page.margin_left_mm));
    print_info.setScalingFactor(1.0);
    print_info.setHorizontallyCentered(false);
    print_info.setVerticallyCentered(false);
    print_info.setHorizontalPagination(NSPrintingPaginationMode::Fit);
    print_info.setVerticalPagination(NSPrintingPaginationMode::Automatic);

    let url = NSURL::fileURLWithPath(&NSString::from_str(&output.to_string_lossy()));
    // SAFETY: claves/valores documentados de NSPrintInfo (NSString → NSURL).
    unsafe {
        print_info.setJobDisposition(NSPrintSaveJob);
        print_info.dictionary().insert(NSPrintJobSavingURL, &url);
    }

    // Fondos: `shouldPrintBackgrounds` existe desde macOS 13.3.
    // SAFETY: se comprueba que el selector exista antes de llamarlo.
    unsafe {
        let preferences = wk.configuration().preferences();
        if preferences.respondsToSelector(sel!(setShouldPrintBackgrounds:)) {
            preferences.setShouldPrintBackgrounds(page.print_background);
        }
    }

    // SAFETY: WKWebView en el hilo principal, macOS 11+.
    let operation = unsafe { wk.printOperationWithPrintInfo(&print_info) };
    operation.setShowsPrintPanel(false);
    operation.setShowsProgressPanel(false);
    operation.setJobTitle(Some(&NSString::from_str("CuentaCobro")));
    if let Some(view) = operation.view() {
        view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), paper));
    }

    let delegate = DELEGATE.with(|cell| cell.get_or_init(|| PrintDelegate::new(mtm)).clone());
    let delegate: &AnyObject = &delegate;
    let job = Box::into_raw(Box::new(PrintJob {
        done,
        _operation: operation.clone(),
    }));

    // SAFETY: el selector coincide con el método definido en PrintDelegate y
    // `job` es un puntero válido que el callback libera.
    unsafe {
        operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
            &window,
            Some(delegate),
            Some(sel!(printOperationDidRun:success:contextInfo:)),
            job.cast(),
        );
    }
}
