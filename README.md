# CuentaCobro

Aplicación de escritorio para crear, imprimir y gestionar cuentas de cobro colombianas. Funciona sin conexión: tus datos viven en una base de datos SQLite local, en tu propio equipo.

Desde la versión 2.0 está construida con **Tauri 2** (backend en **Rust**), **Vue 3.5**, **Pinia** y **Tailwind CSS 4**. Si usabas la versión 1.x, lee [Actualizar desde la versión 1.x](#-actualizar-desde-la-versión-1x).

## 🚀 Características

- **Clientes:** registro de tus clientes, con un informe por cliente que puedes exportar a PDF.
- **Editor de cuentas de cobro:** conceptos, abonos, cuenta bancaria de pago y monto en letras.
- **PDF nativo:** guarda la cuenta de cobro en PDF (tamaño Carta) con el motor de impresión del sistema, o imprímela directamente.
- **Firma:** dibújala una vez en tu perfil, elige su color y decide si va en cada documento.
- **Base de datos local:** SQLite dentro de la app, con respaldo y restauración desde Configuración.
- **Actualizaciones automáticas firmadas:** la app te avisa cuando hay una versión nueva y te pregunta antes de descargarla.
- **Tema claro, oscuro o del sistema** y zoom con Ctrl/Cmd `+`, `-` y `0`.

## 🛠️ Tecnologías

- [Tauri 2](https://tauri.app/) y [Rust](https://www.rust-lang.org/)
- [rusqlite](https://github.com/rusqlite/rusqlite) (SQLite incluido en la app)
- [Vue 3.5](https://vuejs.org/) y [Pinia](https://pinia.vuejs.org/)
- [Vite](https://vite.dev/) y [TypeScript](https://www.typescriptlang.org/)
- [Tailwind CSS 4](https://tailwindcss.com/) y [shadcn-vue](https://www.shadcn-vue.com/) (reka-ui)

## 💻 Requisitos del sistema

- **macOS 12 (Monterey) o superior, con Safari 16.4 o superior.** macOS 13.3 y posteriores ya lo traen. En macOS 12 instala la última actualización de Safari desde Actualización de software: con un Safari anterior la app se ve sin estilos.
- **Windows 10 u 11 (x64 o ARM64).** Si falta Microsoft Edge WebView2, el instalador lo descarga (requiere internet la primera vez).
- **Linux: Ubuntu 22.04+ o Debian 12+ con las actualizaciones al día** (WebKitGTK 2.40 o superior).

## 📥 Instalación

Descarga el instalador de tu sistema desde [GitHub Releases](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/latest). Reemplaza `<versión>` por la versión que descargaste (por ejemplo, `2.0.0`).

| Sistema                          | Archivo                                 |
| -------------------------------- | --------------------------------------- |
| macOS (Apple Silicon e Intel)    | `CuentaCobro_<versión>_universal.dmg`   |
| Windows x64                      | `CuentaCobro_<versión>_x64-setup.exe`   |
| Windows ARM64                    | `CuentaCobro_<versión>_arm64-setup.exe` |
| Ubuntu y Debian                  | `CuentaCobro_<versión>_amd64.deb`       |
| Otras distribuciones Linux (x64) | `CuentaCobro_<versión>_amd64.AppImage`  |

### 🍎 macOS

1.  Abre el `.dmg` y arrastra **CuentaCobro** a la carpeta **Aplicaciones**.
2.  La aplicación no está firmada por Apple. Si al abrirla aparece el error **"CuentaCobro" está dañado y no puede abrirse**, abre la **Terminal** y ejecuta:
    ```bash
    sudo xattr -cr /Applications/CuentaCobro.app
    ```
3.  Introduce tu contraseña de administrador y presiona Enter. Ya puedes abrir la aplicación normalmente.

### 🪟 Windows

Ejecuta `CuentaCobro_<versión>_x64-setup.exe` (o `arm64` en equipos ARM). Se instala solo para tu usuario. Como el instalador no tiene firma Authenticode, SmartScreen puede avisar: elige «Más información» › «Ejecutar de todas formas».

### 🐧 Linux

En Ubuntu o Debian se recomienda el paquete `.deb`. Desde la carpeta de descarga:

```bash
sudo apt install ./CuentaCobro_<versión>_amd64.deb
```

Con el `.AppImage`, dale permiso de ejecución y ábrelo:

```bash
chmod +x CuentaCobro_<versión>_amd64.AppImage
./CuentaCobro_<versión>_amd64.AppImage
```

En Ubuntu 24.04 o superior, el AppImage puede necesitar FUSE 2: `sudo apt install libfuse2t64`.

### 🔔 Actualizaciones

La app busca actualizaciones al arrancar. Si hay una versión nueva, te muestra sus notas y te pregunta antes de descargarla. Al terminar la instalación, el botón «Reiniciar ahora» abre la versión nueva (en Windows, el instalador cierra la app y la vuelve a abrir por su cuenta). También puedes buscarlas en **Configuración → Acerca de → Buscar actualizaciones**. Con el paquete `.deb`, la instalación pide tu contraseña.

### 🤖 CLI para agentes

El instalador incluye `cuentacobro-cli`, un programa de línea de comandos para que un agente de IA (o tú, desde la terminal) consulte y modifique tus datos sin abrir la app. Responde en JSON y usa la misma base de datos y las mismas reglas que la app.

Para usarla desde cualquier terminal, abre la app y ve a **Configuración → CLI para agentes → Instalar en el PATH**:

- **macOS:** crea el comando en `/usr/local/bin` y te pide la contraseña de administrador.
- **Windows:** agrega la carpeta de la app a tu PATH de usuario.
- **Ubuntu y Debian (.deb):** ya viene en el PATH; el botón solo lo confirma. El AppImage no la expone: usa el `.deb`.

Después abre una terminal nueva (o reinicia tu agente) y pídele que ejecute `cuentacobro-cli help`: ahí están los comandos (`query`, `get`, `run`, `cuenta-open`, `cuenta-save` y `backup`) y las reglas que debe seguir. La CLI cambia tus datos reales: pídele que haga un `backup` antes de modificar algo.

Sin instalarla en el PATH también se puede usar con su ruta completa:

| Sistema | Ruta de la CLI                                                 |
| ------- | -------------------------------------------------------------- |
| macOS   | `/Applications/CuentaCobro.app/Contents/MacOS/cuentacobro-cli` |
| Windows | `%LOCALAPPDATA%\CuentaCobro\cuentacobro-cli.exe`               |
| Linux   | `/usr/bin/cuentacobro-cli` (paquete `.deb`)                    |

Si desinstalas la app en macOS, borra también el comando: `sudo rm /usr/local/bin/cuentacobro-cli`.

## 🔄 Actualizar desde la versión 1.x

La versión 1.x (Electron) no se actualiza sola a la 2.x: instálala a mano una vez siguiendo los pasos de tu sistema. Las versiones siguientes ya llegan solas.

- **Windows:** descarga `CuentaCobro_<versión>_x64-setup.exe` (o `arm64`) y ejecútalo. El instalador retira la versión anterior y conserva tus datos. Si la 1.x estaba instalada «para todos los usuarios», desinstálala después desde Configuración › Aplicaciones (aparece como «CuentaCobro 1.0.x»). Si después de instalar sigues viendo la 1.x en Aplicaciones, desinstálala y vuelve a ejecutar el instalador 2.x para recrear los accesos directos. SmartScreen avisará porque el instalador no tiene firma Authenticode: «Más información» › «Ejecutar de todas formas».
- **macOS:** abre `CuentaCobro_<versión>_universal.dmg`, arrastra la app a Aplicaciones y acepta «Reemplazar». Después ejecuta `sudo xattr -cr /Applications/CuentaCobro.app` (el mismo paso de la [instalación en macOS](#-macos)).
- **Ubuntu y Debian (.deb):** `sudo apt install ./CuentaCobro_<versión>_amd64.deb` actualiza el paquete `cuenta-cobro` que ya tenías.
- **AppImage:** descarga el nuevo `.AppImage`, dale permiso con `chmod +x`, ábrelo y borra el anterior. En Ubuntu 24.04 o superior puede requerir `sudo apt install libfuse2t64`.
- **En todos los sistemas:** al abrir la 2.x por primera vez, tus clientes y cuentas de cobro se importan automáticamente y verás el aviso «Datos importados». La importación copia la base de datos de la 1.x sin modificarla ni borrarla. No sigas usando la 1.x: lo que cambies allí ya no aparecerá en la 2.x.
- El tema (claro, oscuro o sistema) vuelve a “Sistema” la primera vez; elígelo de nuevo en Configuración.

## 🗂️ Dónde viven los datos

|                                                  | macOS                                                                                                                           | Windows                                                                                 | Linux                                                                                   |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Datos de la 2.x (`database.sqlite`)              | `~/Library/Application Support/com.cuentacobro.app/`                                                                            | `%APPDATA%\com.cuentacobro.app\`                                                        | `~/.local/share/com.cuentacobro.app/`                                                   |
| Base de datos de la 1.x (se busca en este orden) | `~/Library/Application Support/cuenta-cobro/database.sqlite`, luego `~/Library/Application Support/CuentaCobro/database.sqlite` | `%APPDATA%\cuenta-cobro\database.sqlite`, luego `%APPDATA%\CuentaCobro\database.sqlite` | `~/.config/cuenta-cobro/database.sqlite`, luego `~/.config/CuentaCobro/database.sqlite` |
| Registro de diagnóstico (log)                    | `~/Library/Logs/com.cuentacobro.app/`                                                                                           | `%LOCALAPPDATA%\com.cuentacobro.app\logs\`                                              | `~/.local/share/com.cuentacobro.app/logs/`                                              |

Junto a `database.sqlite` está `legacy-migration.json`, que registra la importación desde la 1.x. La base de datos de la 1.x nunca se modifica ni se borra.

En macOS, Finder muestra la carpeta `com.cuentacobro.app` como si fuera una aplicación (su nombre termina en `.app`). Para copiar tus datos usa Configuración → Crear respaldo. Si necesitas abrir la carpeta, haz clic derecho → «Mostrar contenido del paquete», o en Terminal: `open ~/Library/Application\ Support/com.cuentacobro.app`.

## 👩‍💻 Desarrollo

### Requisitos de desarrollo

- [Node.js](https://nodejs.org/) 24.15 o superior (recomendado, el que usa CI) o 22.22.2 o superior, y npm. Las versiones impares (23 y 25) no son compatibles. El rango exacto está en `engines` de `package.json`.
- [Rust](https://www.rust-lang.org/tools/install) estable 1.90 o superior, instalado con rustup. CI usa la 1.99.1.
- Los [prerrequisitos de Tauri 2](https://v2.tauri.app/start/prerequisites/) de tu sistema:
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`).
  - **Windows:** Microsoft C++ Build Tools (carga de trabajo «Desarrollo para el escritorio con C++») y WebView2.
  - **Linux (Ubuntu/Debian):**
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
      libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```
    Para empaquetar el AppImage hace falta también `patchelf`.

### Pasos para empezar

1.  **Clonar el repositorio:**

    ```bash
    git clone https://github.com/necro304/cuenta_de_cobro_escritorio.git
    cd cuenta_de_cobro_escritorio
    ```

2.  **Instalar dependencias** (exactamente las del `package-lock.json`):

    ```bash
    npm ci
    ```

3.  **Ejecutar en modo desarrollo** (Vite en el puerto 1420 + la app Tauri):

    ```bash
    npm run dev
    ```

    La primera vez compila el backend Rust y tarda unos minutos. `npm run dev` usa la misma carpeta de datos que la app instalada: en el primer arranque importa la base de datos de la 1.x si la encuentra. No se pueden tener abiertas a la vez la app instalada y la de desarrollo: la segunda solo enfoca la primera.

### Comandos

| Comando                  | Qué hace                                                                                                                    |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| `npm run dev`            | Abre la app en modo desarrollo, con recarga en caliente.                                                                    |
| `npm run build`          | Genera los instaladores firmados para el updater. Exige `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. |
| `npm run build:unsigned` | Genera los instaladores sin la firma del updater. Es el que se usa en local.                                                |
| `npm run test:renderer`  | Tests de componentes, stores y capa API (Vitest).                                                                           |
| `npm run test:database`  | Tests del backend Rust (`cargo test`): SQL, cuentas de cobro y migración de datos.                                          |
| `npm run lint`           | ESLint. Falla con cualquier warning.                                                                                        |
| `npm run typecheck`      | Comprobación de tipos con vue-tsc.                                                                                          |

Los instaladores quedan en `src-tauri/target/release/bundle/`. Los releases se publican al subir un tag `v*`: GitHub Actions compila para macOS, Windows y Linux, firma los paquetes del updater y publica el release. Más detalles en [CONTRIBUTING.md](CONTRIBUTING.md).

## 🤝 Contribuir

¡Las contribuciones son bienvenidas! Lee la [guía de contribución](CONTRIBUTING.md) antes de abrir un issue o un pull request.

## 📄 Licencia

Este proyecto está bajo la Licencia MIT - mira el archivo [LICENSE](LICENSE) para detalles.

---

Desarrollado con ❤️ por [necro304](https://github.com/necro304)
