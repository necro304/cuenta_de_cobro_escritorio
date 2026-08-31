import { ref } from 'vue'
import type { Client } from '@/types'

/** Carga la lista de clientes ordenada por nombre. Centraliza la query compartida. */
export function useClients() {
  const clients = ref<Client[]>([])

  const loadClients = async (): Promise<Client[]> => {
    clients.value = await window.electronAPI.dbQuery<Client>(
      'SELECT * FROM clients ORDER BY name ASC',
    )
    return clients.value
  }

  return { clients, loadClients }
}
