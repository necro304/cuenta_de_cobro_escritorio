//! Raíz del backend de CuentaCobro (Tauri 2).
//!
//! Decisiones de la migración desde Electron: docs/adr/0001-migracion-a-tauri.md.

pub mod cli_install;
mod commands;
pub mod cuenta_de_cobro;
pub mod db;
pub mod generic;
pub mod legacy_migration;
pub mod money;
pub mod pdf;
pub mod sqlvalue;
pub mod system;
pub mod types;

use std::sync::{Arc, Mutex, MutexGuard};

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_window_state::StateFlags;

/// Conexión única. `None` = cerrada (durante `db_restore` o si la BD no abrió al arrancar).
#[derive(Clone)]
pub struct Database {
    pub conn: Arc<Mutex<Option<rusqlite::Connection>>>,
    pub path: std::path::PathBuf,
}

impl Database {
    pub fn lock(&self) -> MutexGuard<'_, Option<rusqlite::Connection>> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyMigrationNotice {
    source: String,
    migrated_at_unix: u64,
    /// `macos`, `windows` o `linux` (para adaptar el texto del aviso).
    os: &'static str,
}

/// Devuelve la información de la importación desde Electron UNA sola vez
/// (marca el aviso como mostrado). `None` si no hubo migración o ya se avisó.
#[tauri::command]
fn legacy_migration_notice(app: AppHandle) -> Result<Option<LegacyMigrationNotice>, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = dir.join(legacy_migration::MARKER_FILE_NAME);
    let Ok(bytes) = std::fs::read(&path) else {
        return Ok(None);
    };
    let mut marker: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if marker["noticeAcknowledged"].as_bool().unwrap_or(true) {
        return Ok(None);
    }
    marker["noticeAcknowledged"] = serde_json::Value::Bool(true);
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&marker).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(Some(LegacyMigrationNotice {
        source: marker["source"].as_str().unwrap_or_default().to_owned(),
        migrated_at_unix: marker["migratedAtUnix"].as_u64().unwrap_or_default(),
        os: std::env::consts::OS,
    }))
}

/// Crea la ventana "main" desde su entrada en tauri.conf.json (`"create": false`).
fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .cloned()
        .expect("tauri.conf.json debe declarar la ventana \"main\"");
    tauri::WebviewWindowBuilder::from_config(app, &config)?.build()?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 1) SIEMPRE el primero: evita dos procesos sobre la misma BD. El candado se libera en
        //    cleanup_before_exit, que Tauri ejecuta también antes de app.restart().
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .with_filter(|label| !label.starts_with(pdf::PDF_WINDOW_PREFIX))
                .build(),
        )
        .manage(pdf::PrintReadyRegistry::default())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join(legacy_migration::DB_FILE_NAME);

            // Primer arranque: importar la BD de Electron sin tocar el original. Si existe pero no se
            // pudo importar, NO se crea una BD vacía (impediría reintentar): aviso y salida.
            let candidates = legacy_migration::legacy_candidates(&app.path().config_dir()?);
            if let Err(err) = legacy_migration::migrate_legacy_database(&db_path, &candidates) {
                log::error!("Migración de datos heredados fallida: {err}");
                let handle = app.handle().clone();
                app.dialog()
                    .message(format!(
                        "No se pudieron importar tus datos de la versión anterior de CuentaCobro.\n\n\
                         Tus datos NO se han perdido. Cierra la versión anterior si está abierta \
                         y vuelve a abrir esta aplicación.\n\nDetalle: {err}"
                    ))
                    .title("Error al importar datos")
                    .kind(MessageDialogKind::Error)
                    .buttons(MessageDialogButtons::Ok)
                    .show(move |_| handle.exit(1));
                return Ok(());
            }

            // Si la BD no abre (p. ej. se restauró un archivo que no es SQLite) la app arranca igual con
            // la conexión cerrada: los comandos fallan con ERR_DB_CLOSED y Configuración → Restaurar
            // sigue disponible.
            let conn = match db::open_connection(&db_path).and_then(|c| db::init_db(&c).map(|_| c)) {
                Ok(conn) => Some(conn),
                Err(e) => {
                    let message =
                        format!("No se pudo abrir la base de datos: {}", generic::sqlite_message(e));
                    log::error!("{message}");
                    app.dialog()
                        .message(message)
                        .title("Error de base de datos")
                        .kind(MessageDialogKind::Error)
                        .show(|_| {});
                    None
                }
            };
            app.manage(Database {
                conn: Arc::new(Mutex::new(conn)),
                path: db_path,
            });

            // "create": false en tauri.conf.json: la ventana se crea con la BD ya lista (Tauri crea
            // las ventanas de la config ANTES de llamar a setup).
            create_main_window(app.handle())?;

            #[cfg(debug_assertions)]
            pdf::selftest::maybe_run(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::db_query,
            commands::db_get,
            commands::db_run,
            commands::clear_billing_data,
            commands::cuenta_de_cobro_open,
            commands::cuenta_de_cobro_save,
            system::db_backup,
            system::db_restore,
            system::export_pdf,
            system::export_client_report_pdf,
            system::print_ready,
            legacy_migration_notice,
            cli_install::install_cli,
        ])
        .build(tauri::generate_context!())
        .expect("error al iniciar CuentaCobro")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            match event {
                // Electron: 'window-all-closed' no cierra la app en macOS…
                tauri::RunEvent::ExitRequested {
                    code: None, api, ..
                } => api.prevent_exit(),
                // …y 'activate' recrea la ventana principal si no hay ninguna.
                tauri::RunEvent::Reopen {
                    has_visible_windows: false,
                    ..
                } => {
                    // Sin `Database` gestionada (migración fallida, la app está saliendo) no se recrea.
                    if app.get_webview_window("main").is_none()
                        && app.try_state::<Database>().is_some()
                        && let Err(e) = create_main_window(app)
                    {
                        log::error!("No se pudo recrear la ventana principal: {e}");
                    }
                }
                _ => {}
            }
            // Fuera de macOS no hay nada que hacer; evita `unused_variables` con -D warnings
            // (el `match` de arriba no existe en Windows ni Linux).
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
