//! Configuración → «Instalar en el PATH»: deja disponible en cualquier terminal la `cuentacobro-cli`
//! que los instaladores ponen junto al ejecutable de la app.
use crate::system::OperationResult;
use std::path::{Path, PathBuf};

const MSG_DONE: &str = "Abre una terminal nueva y ejecuta: cuentacobro-cli help";

#[tauri::command]
pub async fn install_cli() -> Result<OperationResult, String> {
    let result =
        tauri::async_runtime::spawn_blocking(|| cli_path().and_then(|cli| add_to_path(&cli)))
            .await
            .map_err(|e| e.to_string())?;
    Ok(match result {
        Ok(message) => OperationResult {
            success: true,
            message,
        },
        Err(message) => OperationResult {
            success: false,
            message,
        },
    })
}

fn cli_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let cli = exe.with_file_name(if cfg!(windows) {
        "cuentacobro-cli.exe"
    } else {
        "cuentacobro-cli"
    });
    if cli.is_file() {
        Ok(cli)
    } else {
        Err(format!(
            "No se encontró la CLI junto a la app: {}",
            cli.display()
        ))
    }
}

/// `path` (un PATH de Windows) con `dir` al final, o `None` si ya estaba. Windows no distingue
/// mayúsculas ni la `\` final.
pub fn path_with_dir(path: &str, dir: &str) -> Option<String> {
    let key = |entry: &str| entry.trim().trim_end_matches('\\').to_lowercase();
    if path.split(';').any(|entry| key(entry) == key(dir)) {
        return None;
    }
    let separator = if path.is_empty() || path.ends_with(';') {
        ""
    } else {
        ";"
    };
    Some(format!("{path}{separator}{dir}"))
}

/// Enlace en /usr/local/bin, que está en el PATH por defecto de macOS. Pide la contraseña de administrador.
#[cfg(target_os = "macos")]
fn add_to_path(cli: &Path) -> Result<String, String> {
    // Si la app no se movió de la descarga, Gatekeeper la ejecuta desde una copia temporal: el enlace se rompería.
    if cli.to_string_lossy().contains("/AppTranslocation/") {
        return Err(
            "Mueve CuentaCobro a la carpeta Aplicaciones, ábrela de nuevo y vuelve a intentarlo."
                .into(),
        );
    }
    let script = r#"do shell script "mkdir -p /usr/local/bin && ln -sf " & quoted form of (item 1 of argv) & " /usr/local/bin/cuentacobro-cli" with prompt "CuentaCobro quiere instalar el comando cuentacobro-cli." with administrator privileges"#;
    let output = std::process::Command::new("osascript")
        .args(["-e", "on run argv", "-e", script, "-e", "end run"])
        .arg(cli)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(MSG_DONE.into());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    // -128: el usuario cerró la ventana de la contraseña.
    Err(if stderr.contains("(-128)") {
        crate::system::MSG_CANCELLED.into()
    } else {
        stderr.trim().to_string()
    })
}

/// El .deb ya la deja en /usr/bin; el AppImage solo la tiene dentro de su montaje temporal.
#[cfg(target_os = "linux")]
fn add_to_path(cli: &Path) -> Result<String, String> {
    let in_path = std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| Some(dir.as_path()) == cli.parent())
    });
    if in_path {
        Ok(MSG_DONE.into())
    } else {
        Err("Con el AppImage la CLI no se puede instalar. Usa el paquete .deb, que ya la deja en el PATH.".into())
    }
}

/// Añade la carpeta de instalación al PATH del usuario (HKCU\Environment) sin tocar el resto.
#[cfg(windows)]
fn add_to_path(cli: &Path) -> Result<String, String> {
    use std::ffi::OsString;
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, REG_EXPAND_SZ, REG_SZ},
        types::{FromRegValue, ToRegValue},
    };
    let dir = cli
        .parent()
        .and_then(Path::to_str)
        .ok_or("La carpeta de instalación tiene un nombre no válido.")?;
    let unreadable = || {
        format!(
            "No se modificó tu PATH porque tiene un formato inesperado. Agrega a mano esta carpeta: {dir}"
        )
    };
    let (env, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey("Environment")
        .map_err(|e| e.to_string())?;
    let (current, vtype) = match env.get_raw_value("Path") {
        Ok(raw) if matches!(raw.vtype, REG_SZ | REG_EXPAND_SZ) => {
            // Estricto: un PATH con UTF-16 inválido se reescribiría dañado.
            let current = OsString::from_reg_value(&raw)
                .map_err(|e| e.to_string())?
                .into_string()
                .map_err(|_| unreadable())?;
            (current, raw.vtype)
        }
        Ok(_) => return Err(unreadable()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (String::new(), REG_EXPAND_SZ),
        Err(e) => return Err(e.to_string()),
    };
    if let Some(path) = path_with_dir(&current, dir) {
        // Conserva el tipo: con REG_SZ dejarían de expandirse entradas como %USERPROFILE%\bin.
        let mut raw = path.to_reg_value();
        raw.vtype = vtype;
        env.set_raw_value("Path", &raw).map_err(|e| e.to_string())?;
        notify_environment_change();
    }
    Ok(MSG_DONE.into())
}

/// Explorer relee el entorno: las terminales que se abran desde ahora ya ven el PATH nuevo.
#[cfg(windows)]
fn notify_environment_change() {
    use windows::Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
        },
    };
    let area: Vec<u16> = "Environment\0".encode_utf16().collect();
    let _ = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(area.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::path_with_dir;

    #[test]
    fn path_with_dir_agrega_al_final_sin_duplicar() {
        let dir = r"C:\Users\Ana\AppData\Local\CuentaCobro";
        assert_eq!(path_with_dir("", dir).as_deref(), Some(dir));
        assert_eq!(
            path_with_dir(r"%USERPROFILE%\bin;C:\x", dir),
            Some(format!(r"%USERPROFILE%\bin;C:\x;{dir}"))
        );
        assert_eq!(path_with_dir(r"C:\x;", dir), Some(format!(r"C:\x;{dir}")));
        assert_eq!(
            path_with_dir(r"C:\x; c:\users\ana\appdata\local\cuentacobro\ ", dir),
            None
        );
    }
}
