# ¿Hay un .deb para Ubuntu en los releases?

**Consulta:** 2026-10-19 UTC, según el reloj del entorno. Solo fuentes primarias: API de GitHub y archivos locales del proyecto.

## Resultado

**No hay ningún asset con extensión `.deb` en los 7 releases devueltos por la API (71 assets en total).** Todos incluyen un AppImage como paquete Linux, no un paquete Debian. La revisión se hizo mediante `gh api`, con paginación automática y `per_page=100`; se obtuvo una única página. [Listado consultado](https://api.github.com/repos/necro304/cuenta_de_cobro_escritorio/releases?per_page=100).

El endpoint [latest](https://api.github.com/repos/necro304/cuenta_de_cobro_escritorio/releases/latest) devuelve **v1.0.10**, publicado el **2026-09-02T19:06:47Z**, sin marca de borrador ni prerelease. Sus assets Linux relevantes son:

- [CuentaCobro-Linux-1.0.10.AppImage](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/download/v1.0.10/CuentaCobro-Linux-1.0.10.AppImage), 135.843.481 bytes.
- [latest-linux.yml](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/download/v1.0.10/latest-linux.yml), metadatos de actualización; no es un instalador `.deb`.

Fuente del release: [página v1.0.10](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.10) y [API por ID](https://api.github.com/repos/necro304/cuenta_de_cobro_escritorio/releases/381503438).

## Cobertura de releases

Se revisaron los nombres de **todos los assets** de cada release listado. Los siete tienen `draft=false` y `prerelease=false`. Fuente: [API paginada](https://api.github.com/repos/necro304/cuenta_de_cobro_escritorio/releases?per_page=100).

| Release | Assets totales | Paquete Linux | Assets `.deb` |
| --- | ---: | --- | ---: |
| [v1.0.10](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.10) | 14 | `CuentaCobro-Linux-1.0.10.AppImage` | 0 |
| [v1.0.9](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.9) | 14 | `CuentaCobro-Linux-1.0.9.AppImage` | 0 |
| [v1.0.8](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.8) | 14 | `CuentaCobro-Linux-1.0.8.AppImage` | 0 |
| [v1.0.7](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.7) | 14 | `CuentaCobro-Linux-1.0.7.AppImage` | 0 |
| [v1.0.6](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.6) | 5 | `CuentaCobro-Linux-1.0.6.AppImage` | 0 |
| [v1.0.5](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.5) | 5 | `CuentaCobro-Linux-1.0.5.AppImage` | 0 |
| [v1.0.4](https://github.com/necro304/cuenta_de_cobro_escritorio/releases/tag/v1.0.4) | 5 | `CuentaCobro-Linux-1.0.4.AppImage` | 0 |

## Evidencia local

- [`electron-builder.json5`](../../electron-builder.json5), líneas 7–12 y 38–40: publica en `necro304/cuenta_de_cobro_escritorio`; Linux configura únicamente `target: ['AppImage']` y nombre `${productName}-Linux-${version}.${ext}`. No configura `deb`.
- [`.github/workflows/release.yml`](../../.github/workflows/release.yml), líneas 3–6, 15–17 y 35–38: se dispara con tags `v*`, usa matriz Ubuntu/Windows/macOS y ejecuta `npm run build -- --publish always`. Ejecutar el job en Ubuntu no implica generar `.deb`.
- [`package.json`](../../package.json), líneas 3 y 18: versión local `1.0.10`; `build` ejecuta `vue-tsc && vite build && electron-builder`.
- [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml), líneas 38–41: la etapa de compilación del CI normal ejecuta `npx vite build`; no contiene una etapa explícita de empaquetado/publicación de releases.

## Acceso y límites

Sin problemas de acceso a la API. Una incompatibilidad inicial de flags del CLI (`--slurp` con `--jq`) se resolvió usando `--paginate --jq`.

Se comprobaron metadatos y nombres, sin descargar ni ejecutar binarios, inspeccionar el contenido de los assets o probar compatibilidad con Ubuntu. La conclusión se limita a los releases visibles devueltos por GitHub en esta consulta; no cubre archivos eliminados, otras distribuciones externas ni publicaciones futuras. La configuración local respalda el resultado, pero no demuestra por sí sola cómo se construyeron versiones históricas. No se modificó configuración ni se hicieron commits.
