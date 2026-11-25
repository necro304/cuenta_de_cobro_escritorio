import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { dbQuery, dbRun } from '@/api/backend'
import type { BankAccount } from '@/types'

export interface BankAccountInput {
  bank: string
  account_type: string
  account_number: string
}

/** Cuentas bancarias del perfil. Exactamente una puede ser la predeterminada. */
export const useBankAccountsStore = defineStore('bankAccounts', () => {
  const accounts = ref<BankAccount[]>([])

  const defaultAccount = computed(
    () => accounts.value.find((account) => account.is_default === 1) ?? null,
  )

  async function load(): Promise<BankAccount[]> {
    accounts.value = await dbQuery<BankAccount>(
      'SELECT * FROM bank_accounts ORDER BY created_at DESC',
    )
    return accounts.value
  }

  /** Crea (sin id) o actualiza. La primera cuenta creada queda como predeterminada. */
  async function save(input: BankAccountInput, id?: number): Promise<void> {
    if (id) {
      await dbRun(
        'UPDATE bank_accounts SET bank = ?, account_type = ?, account_number = ? WHERE id = ?',
        [input.bank, input.account_type, input.account_number, id],
      )
    } else {
      await dbRun(
        'INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, (SELECT COUNT(*) = 0 FROM bank_accounts))',
        [input.bank, input.account_type, input.account_number],
      )
    }
    await load()
  }

  async function remove(id: number): Promise<void> {
    await dbRun('DELETE FROM bank_accounts WHERE id = ?', [id])
    await load()
  }

  /** Una sola sentencia: no puede quedar ninguna o más de una predeterminada. */
  async function setDefault(id: number): Promise<void> {
    await dbRun('UPDATE bank_accounts SET is_default = CASE WHEN id = ? THEN 1 ELSE 0 END', [id])
    await load()
  }

  return { accounts, defaultAccount, load, save, remove, setDefault }
})
