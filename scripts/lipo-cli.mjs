// beforeBundleCommand de Tauri. Con --target universal-apple-darwin, tauri build solo une con lipo el
// binario de la app; sin este paso, el bundle de macOS falla al copiar la CLI. En el resto, no hace nada.
import { execFileSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

if (process.env.TAURI_ENV_TARGET_TRIPLE === 'universal-apple-darwin') {
  const targetDir =
    process.env.CARGO_TARGET_DIR ?? fileURLToPath(new URL('../src-tauri/target', import.meta.url))
  const profile = process.env.TAURI_ENV_DEBUG === 'true' ? 'debug' : 'release'
  const cli = (triple) => path.join(targetDir, triple, profile, 'cuentacobro-cli')
  execFileSync(
    'lipo',
    [
      '-create',
      '-output',
      cli('universal-apple-darwin'),
      cli('aarch64-apple-darwin'),
      cli('x86_64-apple-darwin'),
    ],
    { stdio: 'inherit' },
  )
}
