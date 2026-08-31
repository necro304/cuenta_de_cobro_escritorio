import { ref } from 'vue'
import type { Profile } from '@/types'

/**
 * Carga el perfil singleton (id = 1). Centraliza la query repetida en varias vistas.
 * `loadProfile` actualiza el ref `profile` y devuelve la fila cruda (o undefined).
 */
export function useProfile() {
  const profile = ref<Partial<Profile>>({})

  const loadProfile = async (): Promise<Profile | undefined> => {
    const data = await window.electronAPI.dbGet<Profile>('SELECT * FROM profile WHERE id = 1')
    if (data) profile.value = data
    return data
  }

  return { profile, loadProfile }
}
