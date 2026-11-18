//! CLI de CuentaCobro para agentes: opera la misma BD que la app y responde en JSON.
//! Llama a las mismas funciones que los comandos de Tauri (db_query/db_get/db_run y
//! cuenta_de_cobro), así que aplica las mismas reglas y devuelve los mismos errores que ve el renderer.
use cuentacobro_lib::{cuenta_de_cobro, db, generic, legacy_migration::DB_FILE_NAME, types::*};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

/// `identifier` de tauri.conf.json: la app guarda sus datos en `data_dir()/<identifier>`.
const APP_IDENTIFIER: &str = "com.cuentacobro.app";

const USAGE: &str = r#"cuentacobro-cli: opera los datos de CuentaCobro y responde en JSON.

Uso: cuentacobro-cli [--db <ruta>] <comando> [argumentos]

Sin --db usa la BD de la app (database.sqlite en la carpeta de datos de com.cuentacobro.app)
y falla si todavía no existe. Con --db crea la BD si hace falta: úsalo para pruebas.

Comandos:
  query <sql> [params]   Todas las filas, como dbQuery: [{"columna": valor}, ...]
  get <sql> [params]     La primera fila o null, como dbGet.
  run <sql> [params]     INSERT/UPDATE/DELETE, como dbRun: {"changes", "lastInsertRowid"}.
  cuenta-open [id]       Borrador para crear una cuenta de cobro (trae el siguiente número) o
                         para editar la cuenta <id>, más clientes, cuentas bancarias y resumen.
  cuenta-save <json>     Crea o edita una cuenta de cobro con las reglas de la app.
  cuenta-list            Libro de cuentas con total, abonos, saldo y estado calculados en centavos.
  abono-add <json>       Registra un abono y actualiza el estado en una transacción.
  abono-delete <cuenta> <abono>   Elimina el abono de esa cuenta y recalcula su estado.
  backup <destino>       Copia consistente de la BD. No sobrescribe un archivo existente.

[params] es un arreglo JSON, p. ej. '[1, "texto", null]'. Sin booleanos: usa 1 o 0.

cuenta-save recibe {"kind":"create","cuenta":{...}} o {"kind":"edit","id":<id>,"cuenta":{...}}.
"cuenta" tiene la forma del borrador de cuenta-open; los montos van como texto:
  {"number":"12","date":"2026-11-11","clientId":1,"bankAccountId":1,"notes":"",
   "concepts":[{"description":"Servicio","quantity":"1","price":"150000.50"}]}
cuenta-open y cuenta-save responden {"ok":true,"value":...} o {"ok":false,"error":{"code":...}}.

abono-add recibe {"invoiceId":1,"date":"2026-11-11","amount":"125.50","notes":"Transferencia"}.
El monto admite hasta dos decimales. notes es opcional. abono-add y abono-delete devuelven el
resumen de la cuenta con total, paidAmount, balance y paymentStatus.

Reglas de la app que run no aplica por ti:
  - Crear o editar una cuenta de cobro (número, fecha, cliente, conceptos) solo con cuenta-save:
    calcula el total y el estado y protege los abonos. No escribas en invoice_items.
  - Usa abono-add y abono-delete para mantener el estado sincronizado con los pagos.
  - El perfil es una sola fila: WHERE id = 1.
  - Esquema: query "SELECT sql FROM sqlite_master WHERE type = 'table'"

Si algo falla, el código de salida es 1 y el mensaje va a stderr. Excepción: el {"ok":false} de
cuenta-open y cuenta-save sale por stdout. Si la app está abierta, cambia de vista para ver los cambios.
"#;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (db_path, args) = match args.as_slice() {
        [flag, path, rest @ ..] if flag == "--db" => (Some(PathBuf::from(path)), rest),
        rest => (None, rest),
    };
    let Some((command, args)) = args
        .split_first()
        .filter(|(command, _)| !matches!(command.as_str(), "help" | "--help" | "-h"))
    else {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    };
    match execute(db_path.as_deref(), command, args) {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
            // Solo cuenta-open/save llevan "ok"; SQLite no devuelve booleanos, así que una fila nunca trae ok = false.
            if value["ok"] == false {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(message) => {
            eprintln!("Error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn execute(db_path: Option<&Path>, command: &str, args: &[String]) -> Result<Value, String> {
    match (command, args) {
        ("query" | "get" | "run", [sql, params @ ..]) => {
            let params = parse_params(params)?;
            let conn = open(db_path)?;
            match command {
                "query" => to_json(generic::db_query(&conn, sql, params)?),
                "get" => to_json(generic::db_get(&conn, sql, params)?),
                _ => to_json(generic::db_run(&conn, sql, params)?),
            }
        }
        ("cuenta-list", []) => to_json(
            cuenta_de_cobro::abonos::ledger(&mut open(db_path)?)
                .map_err(generic::sqlite_message)?,
        ),
        ("abono-add", [input]) => {
            let payment = serde_json::from_str(input)
                .map_err(|e| format!("el JSON de abono-add no es válido: {e}"))?;
            to_json(cuenta_de_cobro::abonos::save(&mut open(db_path)?, payment)?)
        }
        ("abono-delete", [invoice_id, payment_id]) => {
            let invoice_id = invoice_id.parse().map_err(|_| "id de cuenta no válido")?;
            let payment_id = payment_id.parse().map_err(|_| "id de abono no válido")?;
            to_json(cuenta_de_cobro::abonos::delete(
                &mut open(db_path)?,
                invoice_id,
                payment_id,
            )?)
        }
        ("cuenta-open", []) => to_json(cuenta_de_cobro::open(
            &open(db_path)?,
            CuentaDeCobroTarget::Create,
        )),
        ("cuenta-open", [id]) => {
            let id = id.parse().map_err(|_| format!("id no válido: {id}"))?;
            to_json(cuenta_de_cobro::open(
                &open(db_path)?,
                CuentaDeCobroTarget::Edit {
                    id: JsNum(Some(id)),
                },
            ))
        }
        ("cuenta-save", [input]) => {
            let input = serde_json::from_str(input)
                .map_err(|e| format!("el JSON de cuenta-save no es válido: {e}"))?;
            to_json(cuenta_de_cobro::save(Some(&mut open(db_path)?), input))
        }
        ("backup", [target]) => {
            // VACUUM INTO saca una copia consistente aunque la app esté escribiendo, y falla si el destino ya existe.
            open(db_path)?
                .execute("VACUUM INTO ?1", [target])
                .map_err(generic::sqlite_message)?;
            Ok(json!({ "path": target }))
        }
        _ => Err(format!(
            "comando o argumentos no válidos: {command}\n\n{USAGE}"
        )),
    }
}

/// Los parámetros van en un solo arreglo JSON, como el segundo argumento de dbQuery/dbGet/dbRun.
fn parse_params(args: &[String]) -> Result<Option<Vec<Value>>, String> {
    match args {
        [] => Ok(None),
        [params] => serde_json::from_str(params)
            .map(Some)
            .map_err(|e| format!("los parámetros deben ser un arreglo JSON: {e}")),
        _ => Err("sobran argumentos: los parámetros van en un solo arreglo JSON".into()),
    }
}

fn to_json(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

fn open(db_path: Option<&Path>) -> Result<rusqlite::Connection, String> {
    let path = match db_path {
        Some(path) => path.to_path_buf(),
        None => {
            let path = dirs::data_dir()
                .ok_or("no se encontró la carpeta de datos del sistema; usa --db <ruta>")?
                .join(APP_IDENTIFIER)
                .join(DB_FILE_NAME);
            // Si la CLI la creara, la app se saltaría la importación de la BD de la 1.x en su primer arranque.
            if !path.exists() {
                return Err(format!(
                    "no existe {}: abre CuentaCobro una vez o usa --db <ruta>",
                    path.display()
                ));
            }
            path
        }
    };
    let conn = db::open_connection(&path).map_err(generic::sqlite_message)?;
    db::init_db(&conn).map_err(generic::sqlite_message)?;
    Ok(conn)
}
