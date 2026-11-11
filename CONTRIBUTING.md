# Guía de Contribución

¡Gracias por tu interés en contribuir a CuentaCobro! Como proyecto de código abierto, valoramos todas las contribuciones, desde la corrección de errores hasta nuevas características y mejoras en la documentación.

## ¿Cómo puedo contribuir?

### Reportar Errores (Bugs)

Si encuentras un error, por favor abre un [issue](https://github.com/necro304/cuenta_de_cobro_escritorio/issues) incluyendo:

- Una descripción clara del error.
- Pasos para reproducirlo.
- Comportamiento esperado vs. comportamiento actual.
- Tu sistema operativo, la versión de CuentaCobro (Configuración → Acerca de) y el instalador que usaste (`.dmg`, `-setup.exe`, `.deb` o `.AppImage`).
- Capturas de pantalla si es posible.

Si la app falla al arrancar o al exportar un PDF, adjunta también el registro de diagnóstico (log). La sección «Dónde viven los datos» del [README](README.md) indica dónde está en cada sistema. Revísalo antes de adjuntarlo: puede incluir rutas de tu equipo.

### Sugerir Mejoras

Las sugerencias para nuevas características son bienvenidas. Abre un issue y describe la funcionalidad que te gustaría ver.

## Configuración del Entorno

1.  Instala los requisitos:
    - [Node.js](https://nodejs.org/) 24.15 o superior (recomendado, el que usa CI) o 22.22.2 o superior, y npm. Las versiones impares (23 y 25) no son compatibles. El rango exacto está en `engines` de `package.json`.
    - [Rust](https://www.rust-lang.org/tools/install) estable 1.90 o superior, con rustup. CI usa la 1.99.1.
    - Los [prerrequisitos de Tauri 2](https://v2.tauri.app/start/prerequisites/) de tu sistema. En Ubuntu o Debian:
      ```bash
      sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
        libssl-dev libayatana-appindicator3-dev librsvg2-dev
      ```
2.  Clona el repositorio e instala las dependencias: `npm ci`.
3.  Abre la app en modo desarrollo: `npm run dev`. La primera compilación del backend Rust tarda unos minutos.

`npm run dev` usa la misma carpeta de datos que la app instalada, y en el primer arranque importa la base de datos de la versión 1.x si la encuentra. Si no quieres tocar tus datos reales, haz un respaldo antes desde Configuración → Crear respaldo. No se pueden tener abiertas a la vez la app instalada y la de desarrollo: la segunda solo enfoca la primera.

## Antes de enviar un Pull Request

Ejecuta estas comprobaciones. Son las mismas que corre CI, y todas deben pasar sin errores ni warnings:

```bash
npm run lint             # ESLint, 0 warnings
npm run typecheck        # vue-tsc
npm run test:renderer    # Vitest

cd src-tauri
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

`cargo` necesita que exista `dist/index.html` para compilar el crate. Si aún no lo tienes, ejecuta antes `npm run build:web`.

Con `--locked`, `cargo` falla si `src-tauri/Cargo.lock` no está al día en vez de reescribirlo sin avisar, igual que en CI. Si cambias dependencias o la versión en `Cargo.toml`, actualiza el lock (sin `--locked`) y súbelo en el mismo commit.

## Flujo de ramas y Pull Requests

1.  El trabajo diario va en la rama `dev` (o en una rama creada desde `dev`).
2.  Los Pull Requests apuntan a `main`.
3.  Escribe mensajes de commit claros y actualiza la documentación si hace falta.

En cada PR a `main`, CI corre el frontend (lint, tipos, tests y build), el backend Rust en Linux, macOS y Windows (`cargo fmt`, `clippy` y `cargo test`) y, además, **empaqueta la app para Windows (NSIS) y Linux (`.deb` y AppImage)**. Los instaladores quedan como artefactos del workflow durante 14 días (`cuentacobro-qa-Windows` y `cuentacobro-qa-Linux`), para probarlos antes de publicar. También puedes lanzar ese empaquetado a mano desde Actions → CI → «Run workflow».

## Estilo de Código

- Usamos **Prettier** para el formateo y **ESLint** para la calidad del código, y `cargo fmt` y `clippy` en Rust.
- Formatea solo los archivos que cambiaste (`npx prettier --write <archivos>`). `npm run format` reformatearía archivos que tu cambio no toca.
- TypeScript sin `any`: usa los tipos de `src/types` o genéricos.
- Código, comentarios y documentación en español, como el resto del proyecto. Los comentarios no deben citar documentos que no estén en el repositorio.
- Las decisiones de la migración a Tauri y las versiones fijadas a propósito están en [docs/adr/0001-migracion-a-tauri.md](docs/adr/0001-migracion-a-tauri.md). Léelo antes de cambiarlas.

## Publicar una versión (mantenedores)

1.  Sube la versión en `package.json` (es la fuente única que usan la app y los instaladores) y deja `version` igual en `src-tauri/Cargo.toml`:
    ```bash
    npm version <versión> --no-git-tag-version      # package.json y package-lock.json
    # edita `version` en src-tauri/Cargo.toml y después actualiza src-tauri/Cargo.lock:
    cd src-tauri && cargo update --workspace --offline
    ```
    Sube `package.json`, `package-lock.json`, `src-tauri/Cargo.toml` y `src-tauri/Cargo.lock` en el mismo commit. Sin el `Cargo.lock` nuevo, el job `rust` de CI falla (`cannot update the lock file … because --locked was passed`).
2.  Escribe las notas en `docs/releases/v<versión>.md`. El workflow las usa como texto del release y como notas del diálogo de actualización de la app (`latest.json`).
3.  Crea y sube el tag `v<versión>`. El workflow **Release** comprueba que el tag coincida con `package.json`, compila para macOS (universal), Windows (x64 y ARM64) y Linux, firma los paquetes del updater, verifica `latest.json`, le copia las notas del release y publica el release.

Para firmar hacen falta los secretos `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` en GitHub, y la clave pública en `plugins.updater.pubkey` de `src-tauri/tauri.conf.json`. Sin la llave privada no se pueden publicar actualizaciones para los usuarios de la 2.x: guárdala junto con su contraseña en un gestor de contraseñas.

¡Esperamos tus contribuciones!
