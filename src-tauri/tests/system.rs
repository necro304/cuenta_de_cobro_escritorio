use cuentacobro_lib::system::{
    client_report_default_name, invoice_pdf_default_name, pdf_target_path,
};
#[test]
fn nombres_por_defecto() {
    assert_eq!(invoice_pdf_default_name(7.0), "cuenta_de_cobro_00007.pdf");
    assert_eq!(
        invoice_pdf_default_name(123456.0),
        "cuenta_de_cobro_123456.pdf"
    );
    assert_eq!(invoice_pdf_default_name(1.5), "cuenta_de_cobro_001.5.pdf");
    assert_eq!(
        client_report_default_name(3.0, "José Ñúñez & Cía. S.A.S."),
        "informe_jose_nunez_cia_s_a_s.pdf"
    );
    assert_eq!(
        client_report_default_name(3.0, "名前"),
        "informe_cliente_3.pdf"
    );
}

/// En Linux GTK confirma la sobrescritura de la ruta escrita (`factura`): la extensión `.pdf`
/// solo se añade si así no se pisa un archivo que el usuario no confirmó.
#[test]
fn ruta_del_pdf_sin_extension_no_pisa_otro_archivo() {
    let dir = tempfile::tempdir().unwrap();
    let elegido = dir.path().join("factura");
    let con_pdf = dir.path().join("factura.pdf");

    assert_eq!(pdf_target_path(elegido.clone()), con_pdf);

    std::fs::write(&con_pdf, b"otro documento").unwrap();
    assert_eq!(pdf_target_path(elegido.clone()), elegido);

    let con_extension = dir.path().join("cuenta.pdf");
    assert_eq!(pdf_target_path(con_extension.clone()), con_extension);
}

/// El renderer (`src/api/backend.ts`, CANCELLED_MESSAGE) compara este texto literal.
#[test]
fn mensaje_de_cancelacion_es_contrato() {
    assert_eq!(
        cuentacobro_lib::system::MSG_CANCELLED,
        "Operación cancelada."
    );
}
