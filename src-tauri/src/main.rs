// Evita la consola extra en Windows en compilaciones release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cuentacobro_lib::run()
}
