<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { Plus, Trash2, ArrowLeft, Check, ChevronsUpDown, RotateCcw } from '@lucide/vue'
import PageHeader from '@/components/PageHeader.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
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
import { calculatePreviewTotal, formatMoneyForDisplay } from '@/lib/cuentaDeCobroMoney'
import type {
  ClienteOption,
  CuentaBancariaOption,
  CuentaDeCobroDiagnostic,
  CuentaDeCobroDraft,
  CuentaDeCobroError,
  CuentaDeCobroValidationIssue,
} from '@/types/cuentaDeCobro'

const router = useRouter()
const route = useRoute()
const { toast } = useToast()

const isEditing = ref(false)
const invoiceIdToEdit = ref<number | null>(null)
const isClientSelectorOpen = ref(false)
const isLoading = ref(true)
const isSaving = ref(false)
const loadError = ref(false)

const clients = ref<ClienteOption[]>([])
const bankAccounts = ref<CuentaBancariaOption[]>([])
const invoice = ref<CuentaDeCobroDraft>({
  number: '',
  date: '',
  clientId: null,
  bankAccountId: null,
  notes: '',
  concepts: [{ description: '', quantity: '1', price: '' }],
})

const total = computed(() => calculatePreviewTotal(invoice.value.concepts))

const selectedBankAccountId = computed({
  get: () => invoice.value.bankAccountId?.toString() ?? '',
  set: (value: string) => {
    invoice.value.bankAccountId = value ? Number(value) : null
  },
})

const describeDiagnostic = (diagnostic: CuentaDeCobroDiagnostic): string => {
  switch (diagnostic.code) {
    case 'HISTORIC_NUMBER_CONFLICT':
      return `El número ${diagnostic.number} también pertenece a otra cuenta.`
    case 'MISSING_CLIENT_REFERENCE':
      return 'El cliente anterior ya no existe; selecciona otro.'
    case 'MISSING_BANK_ACCOUNT_REFERENCE':
      return 'La cuenta bancaria anterior ya no existe; selecciona otra.'
    case 'INVALID_PERSISTED_ACCOUNT':
      return 'Los datos generales guardados deben corregirse antes de actualizar.'
    case 'INVALID_PERSISTED_CONCEPTS':
      return 'Los conceptos guardados deben corregirse antes de actualizar.'
    case 'INVALID_PERSISTED_PAYMENTS':
      return 'Los abonos guardados contienen valores inválidos.'
  }
}

const describeValidationIssue = (issue: CuentaDeCobroValidationIssue): string => {
  switch (issue.code) {
    case 'REQUIRED':
      if (issue.field === 'clientId') return 'Selecciona un cliente.'
      if (issue.field === 'bankAccountId') return 'Selecciona una cuenta bancaria.'
      return 'Todos los conceptos deben tener descripción.'
    case 'INVALID_POSITIVE_INTEGER':
      return 'El número debe ser un entero mayor a 0.'
    case 'INVALID_DATE':
      return 'La fecha no es válida.'
    case 'INVALID_POSITIVE_DECIMAL':
      return issue.field.endsWith('.quantity')
        ? 'Todas las cantidades deben ser mayores a 0.'
        : 'Todos los precios deben ser mayores a 0.'
    case 'AT_LEAST_ONE_CONCEPT_REQUIRED':
      return 'Agrega al menos un concepto válido.'
    case 'SUBTOTAL_ROUNDS_TO_ZERO':
      return 'Cada concepto debe producir un subtotal mínimo de 0,01.'
    case 'AMOUNT_OUT_OF_RANGE':
      return 'Uno de los valores monetarios es demasiado grande o preciso.'
  }
}

const describeSaveError = (error: CuentaDeCobroError): string => {
  switch (error.code) {
    case 'VALIDATION_FAILED':
      return describeValidationIssue(error.issues[0])
    case 'CUENTA_DE_COBRO_NOT_FOUND':
      return 'La cuenta de cobro ya no existe.'
    case 'CLIENTE_NOT_FOUND':
      return 'El cliente seleccionado ya no existe.'
    case 'CUENTA_BANCARIA_NOT_FOUND':
      return 'La cuenta bancaria seleccionada ya no existe.'
    case 'NUMBER_IN_USE':
      return error.suggestedNumber
        ? `El número ${error.number} ya está en uso. Puedes usar el ${error.suggestedNumber}.`
        : `El número ${error.number} ya está en uso.`
    case 'TOTAL_BELOW_PAID_AMOUNT':
      return `El total propuesto (${error.total}) es menor que los abonos registrados (${error.paidAmount}).`
    case 'STORAGE_FAILURE':
      return 'No se pudo guardar la cuenta de cobro. Inténtalo nuevamente.'
  }
}

const copyDraft = (): CuentaDeCobroDraft => ({
  number: invoice.value.number,
  date: invoice.value.date,
  clientId: invoice.value.clientId,
  bankAccountId: invoice.value.bankAccountId,
  notes: invoice.value.notes,
  concepts: invoice.value.concepts.map((concept) => ({ ...concept })),
})

const loadData = async () => {
  isLoading.value = true
  loadError.value = false
  try {
    const idParam = route.params.id
    const id = idParam ? Number(idParam) : null
    isEditing.value = id !== null
    invoiceIdToEdit.value = id

    const result = await window.electronAPI.cuentaDeCobro.open(
      id === null ? { kind: 'create' } : { kind: 'edit', id },
    )

    if (!result.ok) {
      loadError.value = true
      toast({
        title: 'No se pudo abrir el editor',
        description: describeSaveError(result.error),
        variant: 'destructive',
      })
      return
    }

    invoice.value = result.value.cuenta
    clients.value = result.value.clientes
    bankAccounts.value = result.value.cuentasBancarias

    if (result.value.diagnostics.length > 0) {
      toast({
        title: 'Esta cuenta requiere correcciones',
        description: result.value.diagnostics.map(describeDiagnostic).join(' '),
        variant: 'destructive',
      })
    }
  } catch {
    loadError.value = true
    toast({
      title: 'No se pudo abrir el editor',
      description: 'Intenta cargar los datos nuevamente.',
      variant: 'destructive',
    })
  } finally {
    isLoading.value = false
  }
}

const addItem = () => {
  invoice.value.concepts.push({ description: '', quantity: '1', price: '' })
}

const removeItem = (index: number) => {
  invoice.value.concepts.splice(index, 1)
}

const validate = (): string | null => {
  if (!invoice.value.clientId) return 'Selecciona un cliente'
  if (!invoice.value.bankAccountId) return 'Selecciona una cuenta bancaria'
  if (!Number.isSafeInteger(Number(invoice.value.number)) || Number(invoice.value.number) <= 0)
    return 'El número de cuenta debe ser un entero mayor a 0'
  if (!invoice.value.date) return 'La fecha es requerida'
  if (invoice.value.concepts.some((concept) => !concept.description.trim()))
    return 'Todos los conceptos deben tener descripción'
  if (invoice.value.concepts.some((concept) => Number(concept.quantity) <= 0))
    return 'La cantidad de cada concepto debe ser mayor a 0'
  if (invoice.value.concepts.some((concept) => Number(concept.price) <= 0))
    return 'El precio de cada concepto debe ser mayor a 0'
  return null
}

const saveInvoice = async () => {
  const error = validate()
  if (error) {
    toast({ title: 'Error', description: error, variant: 'destructive' })
    return
  }

  isSaving.value = true
  try {
    const result = await window.electronAPI.cuentaDeCobro.save(
      isEditing.value && invoiceIdToEdit.value
        ? { kind: 'edit', id: invoiceIdToEdit.value, cuenta: copyDraft() }
        : { kind: 'create', cuenta: copyDraft() },
    )

    if (!result.ok) {
      toast({
        title: 'No se pudo guardar',
        description: describeSaveError(result.error),
        variant: 'destructive',
      })
      return
    }

    toast({
      title: 'Éxito',
      description: isEditing.value
        ? 'Cuenta de cobro actualizada correctamente'
        : 'Cuenta de cobro guardada correctamente',
    })
    router.push('/invoices')
  } catch {
    toast({
      title: 'Error',
      description: 'No se pudo guardar la cuenta de cobro',
      variant: 'destructive',
    })
  } finally {
    isSaving.value = false
  }
}

onMounted(loadData)
</script>

<template>
  <div class="app-page">
    <PageHeader
      :title="isEditing ? 'Editar cuenta' : 'Nueva cuenta'"
      description="Completa los datos del documento y revisa el total antes de guardar."
    >
      <template #leading>
        <Button variant="ghost" size="sm" class="-ml-3" @click="router.push('/invoices')">
          <ArrowLeft /> Volver a cuentas
        </Button>
      </template>
    </PageHeader>

    <div v-if="isLoading" class="grid gap-6 lg:grid-cols-12" aria-busy="true">
      <div class="surface h-[420px] animate-pulse bg-secondary/70 lg:col-span-8"></div>
      <div class="surface h-64 animate-pulse bg-secondary/70 lg:col-span-4"></div>
    </div>

    <div v-else-if="loadError" class="surface empty-state">
      <div class="empty-state-icon"><RotateCcw /></div>
      <h2 class="section-title">No pudimos cargar los datos</h2>
      <p class="mt-2 text-sm text-muted-foreground">
        Comprueba la base de datos e inténtalo nuevamente.
      </p>
      <Button class="mt-5" variant="outline" @click="loadData"><RotateCcw /> Reintentar</Button>
    </div>

    <div v-else class="grid gap-6 lg:grid-cols-12">
      <div class="space-y-6 lg:col-span-8">
        <div
          v-if="clients.length === 0 || bankAccounts.length === 0"
          class="rounded-lg border border-[hsl(var(--warning)/0.35)] bg-[hsl(var(--warning)/0.08)] p-4 text-sm"
        >
          <p class="font-semibold">Faltan datos para guardar la cuenta</p>
          <ul class="mt-2 space-y-1 text-muted-foreground">
            <li v-if="clients.length === 0">Registra al menos un cliente.</li>
            <li v-if="bankAccounts.length === 0">
              Configura una cuenta bancaria para recibir pagos.
            </li>
          </ul>
          <div class="mt-3 flex flex-wrap gap-2">
            <Button
              v-if="clients.length === 0"
              variant="outline"
              size="sm"
              @click="router.push('/clients')"
            >
              Ir a clientes
            </Button>
            <Button
              v-if="bankAccounts.length === 0"
              variant="outline"
              size="sm"
              @click="router.push('/profile')"
            >
              Configurar cuenta bancaria
            </Button>
          </div>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Detalles</CardTitle>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="grid gap-4 sm:grid-cols-2">
              <div class="grid gap-2">
                <Label for="invoice-number">Número</Label>
                <Input
                  id="invoice-number"
                  :model-value="invoice.number"
                  type="number"
                  @update:model-value="invoice.number = String($event)"
                />
              </div>
              <div class="grid gap-2">
                <Label for="invoice-date">Fecha</Label>
                <Input id="invoice-date" v-model="invoice.date" type="date" />
              </div>
            </div>

            <div class="grid gap-4 sm:grid-cols-2">
              <div class="grid gap-2">
                <Label id="client-label">Cliente</Label>
                <Popover v-model:open="isClientSelectorOpen">
                  <PopoverTrigger as-child>
                    <Button
                      variant="outline"
                      role="combobox"
                      aria-labelledby="client-label"
                      :aria-expanded="isClientSelectorOpen"
                      class="w-full justify-between font-normal"
                    >
                      {{
                        invoice.clientId
                          ? clients.find((client) => client.id === invoice.clientId)?.name
                          : 'Selecciona un cliente...'
                      }}
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
                            @select="
                              () => {
                                invoice.clientId = client.id
                                isClientSelectorOpen = false
                              }
                            "
                          >
                            {{ client.name }}
                            <Check
                              :class="
                                cn(
                                  'ml-auto h-4 w-4',
                                  invoice.clientId === client.id ? 'opacity-100' : 'opacity-0',
                                )
                              "
                            />
                          </CommandItem>
                        </CommandGroup>
                      </CommandList>
                    </Command>
                  </PopoverContent>
                </Popover>
              </div>

              <div class="grid gap-2">
                <Label for="bank-account">Cuenta bancaria para el pago</Label>
                <Select v-model="selectedBankAccountId">
                  <SelectTrigger id="bank-account">
                    <SelectValue placeholder="Selecciona una cuenta" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem
                      v-for="account in bankAccounts"
                      :key="account.id"
                      :value="account.id.toString()"
                    >
                      {{ account.bank }} - {{ account.accountType }} ({{ account.accountNumber }})
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="flex flex-row items-center justify-between gap-4">
            <div>
              <CardTitle>Conceptos</CardTitle>
              <p class="mt-1 text-sm text-muted-foreground">
                Describe cada servicio o producto cobrado.
              </p>
            </div>
            <Button variant="outline" size="sm" @click="addItem" class="gap-1">
              <Plus class="h-4 w-4" /> Agregar
            </Button>
          </CardHeader>
          <CardContent class="space-y-4">
            <div
              v-for="(item, index) in invoice.concepts"
              :key="index"
              class="grid grid-cols-12 gap-3 rounded-lg bg-secondary/55 p-3 sm:gap-4"
            >
              <div class="col-span-12 grid gap-2 sm:col-span-6">
                <Label>Descripción</Label>
                <Input v-model="item.description" placeholder="Ej: Servicio de consultoría..." />
              </div>
              <div class="col-span-4 grid gap-2 sm:col-span-2">
                <Label>Cantidad</Label>
                <Input
                  :model-value="item.quantity"
                  type="number"
                  step="0.01"
                  min="0.01"
                  @update:model-value="item.quantity = String($event)"
                />
              </div>
              <div class="col-span-6 grid gap-2 sm:col-span-3">
                <Label>Precio unitario</Label>
                <Input
                  :model-value="item.price"
                  type="number"
                  step="0.01"
                  min="0.01"
                  @update:model-value="item.price = String($event)"
                />
                <p class="text-xs text-muted-foreground">
                  Subtotal
                  <span class="metric-value font-semibold text-foreground">
                    ${{ formatMoneyForDisplay(calculatePreviewTotal([item])) }}
                  </span>
                </p>
              </div>
              <div class="col-span-2 flex items-end justify-end sm:col-span-1">
                <Button
                  variant="ghost"
                  size="icon"
                  @click="removeItem(index)"
                  :disabled="invoice.concepts.length === 1"
                  aria-label="Eliminar concepto"
                  title="Eliminar concepto"
                >
                  <Trash2 class="h-4 w-4 text-destructive" />
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <div class="space-y-6 lg:col-span-4">
        <Card class="lg:sticky lg:top-0">
          <CardHeader>
            <CardTitle>Resumen de la cuenta</CardTitle>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="rounded-xl bg-accent p-5 text-accent-foreground">
              <p class="text-sm font-medium opacity-80">Total</p>
              <p class="metric-value mt-2 overflow-x-auto whitespace-nowrap text-3xl font-semibold">
                ${{ formatMoneyForDisplay(total) }}
              </p>
            </div>
            <div class="grid gap-2 pt-4 border-t">
              <Label for="invoice-notes">Notas u observaciones</Label>
              <textarea
                id="invoice-notes"
                v-model="invoice.notes"
                class="form-control min-h-[120px] resize-y py-3"
                placeholder="Información opcional para este documento"
              ></textarea>
            </div>
            <Button class="w-full" :disabled="isSaving" @click="saveInvoice">
              {{ isSaving ? 'Guardando...' : isEditing ? 'Actualizar cuenta' : 'Guardar cuenta' }}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
</template>
