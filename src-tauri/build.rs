fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "db_query",
        "db_get",
        "db_run",
        "clear_billing_data",
        "cuenta_de_cobro_open",
        "cuenta_de_cobro_save",
        "invoice_ledger",
        "invoice_payment_history",
        "invoice_payment_save",
        "invoice_payment_delete",
        "db_backup",
        "db_restore",
        "export_pdf",
        "export_client_report_pdf",
        "read_invoice_print_data",
        "read_client_report_data",
        "print_ready",
        "legacy_migration_notice",
        "install_cli",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("no se pudieron preparar los permisos de CuentaCobro");
}
