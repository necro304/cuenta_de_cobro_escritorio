<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { Plus, Trash2, ArrowLeft, Check, ChevronsUpDown } from 'lucide-vue-next'
import ScanningLine from '@/components/ui/animations/ScanningLine.vue'
import ClickBurst from '@/components/ui/animations/ClickBurst.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { cn } from '@/lib/utils'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { useToast } from '@/components/ui/toast/use-toast'
import type { Client, Invoice, InvoiceItem, BankAccount } from '@/types'

type InvoiceItemForm = {
  description: string
  quantity: number
  price: number
}

const router = useRouter()
const route = useRoute()
const { toast } = useToast()

const isEditing = ref(false)
const invoiceIdToEdit = ref<number | null>(null)
const isClientSelectorOpen = ref(false)

const clients = ref<Client[]>([])
const bankAccounts = ref<BankAccount[]>([])
const invoice = ref({
  number: '',
  date: new Date().toISOString().split('T')[0],
  client_id: '',
  bank_account_id: '',
  notes: '',
  items: [{ description: '', quantity: 1, price: 0 }] as InvoiceItemForm[],
})

const total = computed(() => {
  return invoice.value.items.reduce(
    (sum, item) => sum + Number(item.quantity) * Number(item.price),
    0,
  )
})

const loadData = async () => {
  clients.value = await window.electronAPI.dbQuery<Client>('SELECT * FROM clients ORDER BY name ASC')
  bankAccounts.value = await window.electronAPI.dbQuery<BankAccount>('SELECT * FROM bank_accounts ORDER BY is_default DESC, bank ASC')

  const idParam = route.params.id
  if (idParam) {
    isEditing.value = true
    invoiceIdToEdit.value = Number(idParam)
    
    const existingInvoice = await window.electronAPI.dbGet<Invoice>('SELECT * FROM invoices WHERE id = ?', [invoiceIdToEdit.value])
    if (existingInvoice) {
      invoice.value.number = existingInvoice.number.toString()
      invoice.value.date = existingInvoice.date
      invoice.value.client_id = existingInvoice.client_id.toString()
      invoice.value.bank_account_id = existingInvoice.bank_account_id?.toString() || ''
      invoice.value.notes = existingInvoice.notes || ''
      
      const existingItems = await window.electronAPI.dbQuery<InvoiceItem>('SELECT * FROM invoice_items WHERE invoice_id = ?', [invoiceIdToEdit.value])
      if (existingItems && existingItems.length > 0) {
        invoice.value.items = existingItems.map(item => ({
          description: item.description,
          quantity: item.quantity,
          price: item.price
        }))
      }
    }
  } else {
    const lastInvoice = await window.electronAPI.dbGet<{ last: number | null }>(
      'SELECT MAX(number) as last FROM invoices',
    )
    invoice.value.number = ((lastInvoice?.last ?? 0) + 1).toString()

    // Pre-select default bank account
    const defaultAccount = bankAccounts.value.find(acc => acc.is_default)
    if (defaultAccount) {
      invoice.value.bank_account_id = defaultAccount.id.toString()
    }
  }
}

const addItem = () => {
  invoice.value.items.push({ description: '', quantity: 1, price: 0 })
}

const removeItem = (index: number) => {
  invoice.value.items.splice(index, 1)
}

const validate = (): string | null => {
  if (!invoice.value.client_id) return 'Selecciona un cliente'
  if (!invoice.value.bank_account_id) return 'Selecciona una cuenta bancaria'
  if (!invoice.value.number || Number(invoice.value.number) <= 0) return 'El número de cuenta debe ser mayor a 0'
  if (!invoice.value.date) return 'La fecha es requerida'
  if (invoice.value.items.some(item => !item.description.trim())) return 'Todos los ítems deben tener descripción'
  if (invoice.value.items.some(item => Number(item.quantity) <= 0)) return 'La cantidad de cada ítem debe ser mayor a 0'
  if (invoice.value.items.some(item => Number(item.price) < 0)) return 'El precio no puede ser negativo'
  return null
}

const saveInvoice = async () => {
  const error = validate()
  if (error) {
    toast({ title: 'Error', description: error, variant: 'destructive' })
    return
  }

  try {
    if (isEditing.value && invoiceIdToEdit.value) {
      await window.electronAPI.dbRun(
        'UPDATE invoices SET number = ?, date = ?, client_id = ?, bank_account_id = ?, total = ?, notes = ? WHERE id = ?',
        [invoice.value.number, invoice.value.date, invoice.value.client_id, invoice.value.bank_account_id, total.value, invoice.value.notes, invoiceIdToEdit.value]
      )
      
      await window.electronAPI.dbRun('DELETE FROM invoice_items WHERE invoice_id = ?', [invoiceIdToEdit.value])
      
      for (const item of invoice.value.items) {
        await window.electronAPI.dbRun(
          'INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)',
          [invoiceIdToEdit.value, item.description, Number(item.quantity), Number(item.price)],
        )
      }
      toast({ title: 'Éxito', description: 'Cuenta de cobro actualizada correctamente' })
    } else {
      const result = await window.electronAPI.dbRun(
        'INSERT INTO invoices (number, date, client_id, bank_account_id, total, notes) VALUES (?, ?, ?, ?, ?, ?)',
        [invoice.value.number, invoice.value.date, invoice.value.client_id, invoice.value.bank_account_id, total.value, invoice.value.notes],
      )

      const invoiceId = result.lastInsertRowid

      for (const item of invoice.value.items) {
        await window.electronAPI.dbRun(
          'INSERT INTO invoice_items (invoice_id, description, quantity, price) VALUES (?, ?, ?, ?)',
          [invoiceId as number, item.description, Number(item.quantity), Number(item.price)],
        )
      }
      toast({ title: 'Éxito', description: 'Cuenta de cobro guardada correctamente' })
    }
    
    router.push('/invoices')
  } catch {
    toast({ title: 'Error', description: 'No se pudo guardar la cuenta de cobro', variant: 'destructive' })
  }
}

onMounted(loadData)
</script>

<template>
  <div class="space-y-12 pb-10">
    <!-- Header -->
    <div class="border-b-[4px] border-foreground pb-6 mb-8 flex flex-col md:flex-row md:items-end justify-between gap-6 relative overflow-hidden">
      <ScanningLine />
      <div class="flex items-center gap-6">
        <button 
          class="p-4 border-[3px] border-foreground bg-card hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
          @click="router.back()"
        >
          <ArrowLeft class="h-6 w-6" />
        </button>
        <div>
          <h2 class="text-5xl md:text-7xl font-black tracking-tighter uppercase leading-none mb-3">
            {{ isEditing ? 'Editar Cuenta' : 'Nueva Cuenta' }}
          </h2>
          <div class="flex items-center gap-3">
            <div class="h-3 w-3 rounded-full bg-accent border border-foreground animate-pulse"></div>
            <p class="font-mono text-xs font-bold tracking-widest text-muted-foreground uppercase">Editor de Registros / SYS_EDIT</p>
          </div>
        </div>
      </div>
    </div>

    <div class="grid gap-6 md:grid-cols-3">
      <div class="md:col-span-2 space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Detalles</CardTitle>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div class="grid gap-2">
                <Label>Número</Label>
                <Input v-model="invoice.number" type="number" />
              </div>
              <div class="grid gap-2">
                <Label>Fecha</Label>
                <Input v-model="invoice.date" type="date" />
              </div>
            </div>

            <div class="grid grid-cols-2 gap-4">
              <div class="grid gap-2">
                <Label>Cliente</Label>
                <Popover v-model:open="isClientSelectorOpen">
                  <PopoverTrigger as-child>
                    <Button
                      variant="outline"
                      role="combobox"
                      :aria-expanded="isClientSelectorOpen"
                      class="w-full justify-between font-normal"
                    >
                      {{ invoice.client_id
                        ? clients.find((client) => client.id.toString() === invoice.client_id)?.name
                        : "Selecciona un cliente..." }}
                      <ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
                    </Button>
                  </PopoverTrigger>
                  <PopoverContent class="w-full p-0" align="start">
                    <Command>
                      <CommandInput class="h-9" placeholder="Buscar cliente..." />
                      <CommandEmpty>No se encontró ningún cliente.</CommandEmpty>
                      <CommandList>
                        <CommandGroup>
                          <CommandItem
                            v-for="client in clients"
                            :key="client.id"
                            :value="client.name"
                            @select="() => {
                              invoice.client_id = client.id.toString()
                              isClientSelectorOpen = false
                            }"
                          >
                            {{ client.name }}
                            <Check
                              :class="cn(
                                'ml-auto h-4 w-4',
                                invoice.client_id === client.id.toString() ? 'opacity-100' : 'opacity-0',
                              )"
                            />
                          </CommandItem>
                        </CommandGroup>
                      </CommandList>
                    </Command>
                  </PopoverContent>
                </Popover>
              </div>

              <div class="grid gap-2">
                <Label>Cuenta Bancaria para el Pago</Label>
                <Select v-model="invoice.bank_account_id">
                  <SelectTrigger>
                    <SelectValue placeholder="Selecciona una cuenta" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem v-for="account in bankAccounts" :key="account.id" :value="account.id.toString()">
                      {{ account.bank }} - {{ account.account_type }} ({{ account.account_number }})
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="flex flex-row items-center justify-between">
            <CardTitle>Ítems / Conceptos</CardTitle>
            <Button variant="outline" size="sm" @click="addItem" class="gap-1">
              <Plus class="h-4 w-4" /> Agregar Ítem
            </Button>
          </CardHeader>
          <CardContent class="space-y-4">
            <div v-for="(item, index) in invoice.items" :key="index" class="grid grid-cols-12 gap-4 items-end">
              <div class="col-span-6 grid gap-2">
                <Label v-if="index === 0">Descripción</Label>
                <Input v-model="item.description" placeholder="Ej: Servicio de consultoría..." />
              </div>
              <div class="col-span-2 grid gap-2">
                <Label v-if="index === 0">Cant.</Label>
                <Input v-model.number="item.quantity" type="number" step="0.01" min="0.01" />
              </div>
              <div class="col-span-3 grid gap-2">
                <Label v-if="index === 0">Precio Unit.</Label>
                <Input v-model.number="item.price" type="number" min="0" />
              </div>
              <div class="col-span-1">
                <Button variant="ghost" size="icon" @click="removeItem(index)" :disabled="invoice.items.length === 1">
                  <Trash2 class="h-4 w-4 text-destructive" />
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <div class="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Resumen</CardTitle>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="flex justify-between text-lg font-bold">
              <span>Total</span>
              <span>${{ total.toLocaleString() }}</span>
            </div>
            <div class="grid gap-2 pt-4 border-t">
              <Label>Notas / Observaciones</Label>
              <textarea
                v-model="invoice.notes"
                class="min-h-[100px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
              ></textarea>
            </div>
            <Button class="w-full" @click="saveInvoice">{{ isEditing ? 'Actualizar Cuenta de Cobro' : 'Guardar Cuenta de Cobro' }}</Button>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
</template>
