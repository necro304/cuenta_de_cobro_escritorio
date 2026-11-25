import { defineStore } from 'pinia'
import { ref } from 'vue'
import { dbQuery, dbRun } from '@/api/backend'
import type { Client } from '@/types'

/** Campos editables de un cliente en el formulario del directorio. */
export interface ClientInput {
  name: string
  document_id: string
  address: string
  city: string
  phone: string
  email: string
}

/** Directorio de clientes ordenado por nombre. Las escrituras dejan la lista al día. */
export const useClientsStore = defineStore('clients', () => {
  const clients = ref<Client[]>([])

  async function load(): Promise<Client[]> {
    clients.value = await dbQuery<Client>('SELECT * FROM clients ORDER BY name ASC')
    return clients.value
  }

  async function create(input: ClientInput): Promise<number> {
    const { lastInsertRowid } = await dbRun(
      'INSERT INTO clients (name, document_id, address, city, phone, email) VALUES (?, ?, ?, ?, ?, ?)',
      [input.name, input.document_id, input.address, input.city, input.phone, input.email],
    )
    await load()
    return lastInsertRowid
  }

  async function update(id: number, input: ClientInput): Promise<void> {
    await dbRun(
      'UPDATE clients SET name = ?, document_id = ?, address = ?, city = ?, phone = ?, email = ? WHERE id = ?',
      [input.name, input.document_id, input.address, input.city, input.phone, input.email, id],
    )
    await load()
  }

  /** Rechaza (BackendError) si el cliente tiene cuentas de cobro: FK activas en SQLite. */
  async function remove(id: number): Promise<void> {
    await dbRun('DELETE FROM clients WHERE id = ?', [id])
    clients.value = clients.value.filter((client) => client.id !== id)
  }

  return { clients, load, create, update, remove }
})
