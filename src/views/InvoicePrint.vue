<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Printer, ArrowLeft, Download, FileWarning, RotateCcw } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useToast } from '@/components/ui/toast/use-toast'
import { useProfile } from '@/composables/useProfile'
import SignatureAskDialog from '@/components/SignatureAskDialog.vue'
import type { Client, Invoice, InvoiceItem, InvoicePayment, BankAccount, TemplateId } from '@/types'

import DefaultTemplate from '@/components/templates/DefaultTemplate.vue'
import SimpleTemplate from '@/components/templates/SimpleTemplate.vue'

const route = useRoute()
const router = useRouter()
const { toast } = useToast()
const invoiceId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id

// Modo PDF: la ventana oculta del main process carga esta vista con
// ?pdf=1&template=X&signature=0|1 y avisa con notifyPrintReady() al terminar
const isPdfMode = route.query.pdf === '1'

const { profile, loadProfile } = useProfile()
const client = ref<Partial<Client>>({})
const invoice = ref<Partial<Invoice>>({})
const items = ref<InvoiceItem[]>([])
const payments = ref<InvoicePayment[]>([])
const bankAccount = ref<BankAccount | null>(null)

const selectedTemplate = ref<TemplateId>('default')
const includeSignature = ref(false)
// Si el usuario ya eligió manualmente, el modo 'ask' no vuelve a preguntar
const userChoseSignature = ref(false)
const isAskDialogOpen = ref(false)
const pendingAction = ref<'print' | 'pdf' | null>(null)
const isLoading = ref(true)
const loadError = ref(false)
const isExporting = ref(false)

const signatureToRender = computed(() =>
  includeSignature.value ? (profile.value.signature ?? null) : null,
)

const signatureChoice = computed({
  get: () => (includeSignature.value ? 'with' : 'without'),
  set: (value: string) => {
    includeSignature.value = value === 'with'
    userChoseSignature.value = true
  },
})

const asTemplateId = (value: unknown): TemplateId => (value === 'simple' ? 'simple' : 'default')

const loadData = async () => {
  isLoading.value = true
  loadError.value = false
  try {
    const profileData = await loadProfile()
    if (profileData) {
      if (isPdfMode) {
        selectedTemplate.value = asTemplateId(route.query.template)
        includeSignature.value = route.query.signature === '1'
      } else {
        selectedTemplate.value = asTemplateId(profileData.default_template)
        includeSignature.value = profileData.signature_mode === 'auto' && !!profileData.signature
      }
    }

    const invoiceData = await window.electronAPI.dbGet<Invoice>(
      'SELECT * FROM invoices WHERE id = ?',
      [invoiceId],
    )
    if (!invoiceData) {
      loadError.value = true
      return
    }
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
  } catch {
    loadError.value = true
    if (!isPdfMode) {
      toast({
        title: 'No se pudo abrir la cuenta',
        description: 'Intenta cargar la vista previa nuevamente.',
        variant: 'destructive',
      })
    }
  } finally {
    isLoading.value = false
    if (isPdfMode) {
      await nextTick()
      setTimeout(() => window.electronAPI.notifyPrintReady(), 100)
    }
  }
}

const mustAskSignature = () =>
  !isPdfMode &&
  profile.value.signature_mode === 'ask' &&
  !!profile.value.signature &&
  !userChoseSignature.value

const runAction = async (action: 'print' | 'pdf') => {
  if (action === 'print') {
    await nextTick()
    window.print()
    return
  }

  isExporting.value = true
  try {
    const result = await window.electronAPI.exportPdf({
      invoiceId: Number(invoiceId),
      invoiceNumber: invoice.value.number ?? 0,
      template: selectedTemplate.value,
      includeSignature: includeSignature.value,
    })
    if (result.success) {
      toast({ title: 'PDF descargado', description: result.message })
    } else if (result.message !== 'Operación cancelada.') {
      toast({ title: 'Error', description: result.message, variant: 'destructive' })
    }
  } finally {
    isExporting.value = false
  }
}

const requestAction = (action: 'print' | 'pdf') => {
  if (mustAskSignature()) {
    pendingAction.value = action
    isAskDialogOpen.value = true
    return
  }
  runAction(action)
}

const onSignatureChoice = (withSignature: boolean) => {
  includeSignature.value = withSignature
  userChoseSignature.value = true
  const action = pendingAction.value
  pendingAction.value = null
  if (action) runAction(action)
}

// El documento se imprime siempre en claro: se quita la clase dark mientras
// esta vista está montada, sin tocar la preferencia persistida del tema
let hadDarkClass = false

onMounted(() => {
  hadDarkClass = document.documentElement.classList.contains('dark')
  document.documentElement.classList.remove('dark')
  loadData()
})

onUnmounted(() => {
  if (hadDarkClass) {
    document.documentElement.classList.add('dark')
  }
})
</script>

<template>
  <div class="print-container h-dvh min-h-[100dvh] overflow-y-auto bg-secondary/60">
    <div
      v-if="!isPdfMode"
      class="print-controls sticky top-0 z-10 flex flex-col gap-3 border-b bg-card/95 px-4 py-3 backdrop-blur-xl sm:flex-row sm:items-center sm:justify-between no-print"
    >
      <Button variant="ghost" @click="router.push('/invoices')" class="self-start gap-2">
        <ArrowLeft class="h-4 w-4" />
        Cuentas
      </Button>

      <div class="flex flex-wrap items-center gap-2">
        <label class="flex items-center gap-2">
          <span class="sr-only">Plantilla</span>
          <Select v-model="selectedTemplate">
            <SelectTrigger class="w-[160px] bg-background">
              <SelectValue placeholder="Seleccionar plantilla" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="default">Por defecto</SelectItem>
                <SelectItem value="simple">Simple</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </label>

        <label v-if="profile.signature" class="flex items-center gap-2">
          <span class="sr-only">Firma</span>
          <Select v-model="signatureChoice">
            <SelectTrigger class="w-[130px] bg-background">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="with">Con firma</SelectItem>
                <SelectItem value="without">Sin firma</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </label>

        <Button
          variant="outline"
          :disabled="isLoading || loadError || isExporting"
          @click="requestAction('pdf')"
          class="gap-2"
        >
          <Download class="h-4 w-4" />
          {{ isExporting ? 'Generando...' : 'Descargar PDF' }}
        </Button>

        <Button :disabled="isLoading || loadError" @click="requestAction('print')" class="gap-2">
          <Printer class="h-4 w-4" />
          Imprimir
        </Button>
      </div>
    </div>

    <main
      class="document-stage mx-auto min-h-[calc(100dvh-69px)] max-w-[1100px] p-4 sm:p-8"
      :class="isPdfMode ? '!max-w-none !p-0' : ''"
    >
      <div
        v-if="isLoading"
        class="mx-auto h-[900px] max-w-[816px] animate-pulse bg-white shadow-sm"
      ></div>
      <div
        v-else-if="loadError && !isPdfMode"
        class="mx-auto mt-16 flex max-w-lg flex-col items-center rounded-xl border bg-card p-10 text-center shadow-sm"
      >
        <div class="empty-state-icon"><FileWarning /></div>
        <h1 class="section-title">No encontramos esta cuenta</h1>
        <p class="mt-2 text-sm text-muted-foreground">
          El registro no existe o no se pudo consultar.
        </p>
        <Button class="mt-5" variant="outline" @click="loadData"><RotateCcw /> Reintentar</Button>
      </div>
      <template v-else-if="!loadError">
        <DefaultTemplate
          v-if="selectedTemplate === 'default'"
          :profile="profile"
          :client="client"
          :invoice="invoice"
          :items="items"
          :bankAccount="bankAccount"
          :signature="signatureToRender"
        />
        <SimpleTemplate
          v-if="selectedTemplate === 'simple'"
          :profile="profile"
          :client="client"
          :invoice="invoice"
          :items="items"
          :bankAccount="bankAccount"
          :signature="signatureToRender"
        />
      </template>
    </main>

    <SignatureAskDialog v-model:open="isAskDialogOpen" @choice="onSignatureChoice" />
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
  .document-stage {
    padding: 0 !important;
    min-height: auto !important;
  }
  @page {
    margin: 1.5cm;
  }
}
</style>
