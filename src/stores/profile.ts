import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { dbGet, dbRun } from '@/api/backend'
import type { Profile, SignatureMode, TemplateId } from '@/types'

/** Datos personales editables en Mi perfil (todas cadenas, nunca null). */
export interface ProfileDetails {
  name: string
  document_type: string
  document_id: string
  rut: string
  address: string
  phone: string
  email: string
  bank_info: string
}

export interface ProfilePreferences {
  signatureMode: SignatureMode
  defaultTemplate: TemplateId
}

/** Perfil singleton (fila id = 1). Toda lectura/escritura del perfil pasa por aquí. */
export const useProfileStore = defineStore('profile', () => {
  const profile = ref<Profile | null>(null)

  const signature = computed(() => profile.value?.signature ?? null)
  const signatureMode = computed<SignatureMode>(() => profile.value?.signature_mode ?? 'auto')
  const defaultTemplate = computed<TemplateId>(() =>
    profile.value?.default_template === 'simple' ? 'simple' : 'default',
  )
  /** Modo 'auto' con firma guardada: imprimir/exportar con firma sin preguntar. */
  const signsByDefault = computed(() => signatureMode.value === 'auto' && !!signature.value)
  /** Modo 'ask' con firma guardada: preguntar antes de imprimir/exportar. */
  const asksForSignature = computed(() => signatureMode.value === 'ask' && !!signature.value)

  /** Relee siempre desde la base (barato) y devuelve la fila cruda. */
  async function load(): Promise<Profile | undefined> {
    const data = await dbGet<Profile>('SELECT * FROM profile WHERE id = 1')
    if (data) profile.value = data
    return data
  }

  async function saveDetails(details: ProfileDetails): Promise<void> {
    await dbRun(
      'UPDATE profile SET name = ?, document_type = ?, document_id = ?, rut = ?, address = ?, phone = ?, email = ?, bank_info = ? WHERE id = 1',
      [
        details.name,
        details.document_type,
        details.document_id,
        details.rut,
        details.address,
        details.phone,
        details.email,
        details.bank_info,
      ],
    )
    if (profile.value) Object.assign(profile.value, details)
  }

  async function saveSignature(dataUrl: string | null, color: string): Promise<void> {
    await dbRun('UPDATE profile SET signature = ?, signature_color = ? WHERE id = 1', [
      dataUrl,
      color,
    ])
    if (profile.value) {
      profile.value.signature = dataUrl
      profile.value.signature_color = color
    }
  }

  async function savePreferences(preferences: ProfilePreferences): Promise<void> {
    await dbRun('UPDATE profile SET signature_mode = ?, default_template = ? WHERE id = 1', [
      preferences.signatureMode,
      preferences.defaultTemplate,
    ])
    if (profile.value) {
      profile.value.signature_mode = preferences.signatureMode
      profile.value.default_template = preferences.defaultTemplate
    }
  }

  return {
    profile,
    signature,
    signatureMode,
    defaultTemplate,
    signsByDefault,
    asksForSignature,
    load,
    saveDetails,
    saveSignature,
    savePreferences,
  }
})
