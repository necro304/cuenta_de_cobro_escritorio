<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Printer, ArrowLeft } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { Profile, Client, Invoice, InvoiceItem, InvoicePayment, BankAccount } from '@/types'

import DefaultTemplate from '@/components/templates/DefaultTemplate.vue'
import SimpleTemplate from '@/components/templates/SimpleTemplate.vue'

const route = useRoute()
const router = useRouter()
const invoiceId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id

const profile = ref<Partial<Profile>>({})
const client = ref<Partial<Client>>({})
const invoice = ref<Partial<Invoice>>({})
const items = ref<InvoiceItem[]>([])
const payments = ref<InvoicePayment[]>([])
const bankAccount = ref<BankAccount | null>(null)

const selectedTemplate = ref('default')

const loadData = async () => {
  const profileData = await window.electronAPI.dbGet<Profile>('SELECT * FROM profile WHERE id = 1')
  if (profileData) profile.value = profileData

  const invoiceData = await window.electronAPI.dbGet<Invoice>(
    'SELECT * FROM invoices WHERE id = ?',
    [invoiceId],
  )
  if (invoiceData) {
    invoice.value = invoiceData

    if (invoiceData.bank_account_id) {
      const bankData = await window.electronAPI.dbGet<BankAccount>(
        'SELECT * FROM bank_accounts WHERE id = ?',
        [invoiceData.bank_account_id],
      )
      if (bankData) bankAccount.value = bankData
    }

    const clientData = await window.electronAPI.dbGet<Client>(
      'SELECT * FROM clients WHERE id = ?',
      [invoiceData.client_id],
    )
    if (clientData) client.value = clientData

    items.value = await window.electronAPI.dbQuery<InvoiceItem>(
      'SELECT * FROM invoice_items WHERE invoice_id = ?',
      [invoiceId],
    )

    payments.value = await window.electronAPI.dbQuery<InvoicePayment>(
      'SELECT * FROM invoice_payments WHERE invoice_id = ? ORDER BY date ASC',
      [invoiceId],
    )
  }
}

const print = () => {
  window.print()
}

onMounted(loadData)
</script>

<template>
  <div class="print-container min-h-screen bg-white">
    <!-- Controls (hidden when printing) -->
    <div class="print-controls p-4 bg-muted/50 border-b flex justify-between items-center no-print">
      <Button variant="outline" @click="router.back()" class="gap-2">
        <ArrowLeft class="h-4 w-4" />
        Volver
      </Button>

      <div class="flex items-center gap-4">
        <div class="flex items-center gap-2">
          <span class="text-sm font-medium">Plantilla:</span>
          <Select v-model="selectedTemplate">
            <SelectTrigger class="w-[180px] bg-white">
              <SelectValue placeholder="Seleccionar plantilla" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="default">Por defecto</SelectItem>
                <SelectItem value="simple">Simple</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <Button @click="print" class="gap-2">
          <Printer class="h-4 w-4" />
          Imprimir
        </Button>
      </div>
    </div>

    <!-- Invoice Content -->
    <DefaultTemplate
      v-if="selectedTemplate === 'default'"
      :profile="profile"
      :client="client"
      :invoice="invoice"
      :items="items"
      :bankAccount="bankAccount"
    />
    <SimpleTemplate
      v-if="selectedTemplate === 'simple'"
      :profile="profile"
      :client="client"
      :invoice="invoice"
      :items="items"
      :bankAccount="bankAccount"
    />
  </div>
</template>

<style scoped>
@media print {
  .no-print {
    display: none !important;
  }
  .print-container {
    background: transparent !important;
    min-height: auto !important;
  }
  @page {
    margin: 1.5cm;
  }
}
</style>
