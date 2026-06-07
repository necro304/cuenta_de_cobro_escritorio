<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Printer, ArrowLeft, Download } from '@lucide/vue'
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
import SignatureAskDialog from '@/components/SignatureAskDialog.vue'
import type {
  Profile,
  Client,
  Invoice,
  InvoiceItem,
  InvoicePayment,
  BankAccount,
  TemplateId,
} from '@/types'

import DefaultTemplate from '@/components/templates/DefaultTemplate.vue'
import SimpleTemplate from '@/components/templates/SimpleTemplate.vue'

const route = useRoute()
const router = useRouter()
const { toast } = useToast()
const invoiceId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id

// Modo PDF: la ventana oculta del main process carga esta vista con
// ?pdf=1&template=X&signature=0|1 y avisa con notifyPrintReady() al terminar
const isPdfMode = route.query.pdf === '1'

const profile = ref<Partial<Profile>>({})
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
  const profileData = await window.electronAPI.dbGet<Profile>('SELECT * FROM profile WHERE id = 1')
  if (profileData) {
    profile.value = profileData

    if (isPdfMode) {
      selectedTemplate.value = asTemplateId(route.query.template)
      includeSignature.value = route.query.signature === '1'
    } else {
      selectedTemplate.value = asTemplateId(profileData.default_template)
      // En modo 'ask' la vista previa se muestra sin firma hasta imprimir/descargar
      includeSignature.value = profileData.signature_mode === 'auto' && !!profileData.signature
    }
  }

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

  if (isPdfMode) {
    await nextTick()
    // Margen extra para que la imagen de la firma termine de pintarse
    setTimeout(() => window.electronAPI.notifyPrintReady(), 100)
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
  <div class="print-container min-h-screen bg-white">
    <!-- Controls (hidden when printing or rendering for PDF) -->
    <div
      v-if="!isPdfMode"
      class="print-controls p-4 bg-muted/50 border-b flex justify-between items-center no-print"
    >
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

        <div v-if="profile.signature" class="flex items-center gap-2">
          <span class="text-sm font-medium">Firma:</span>
          <Select v-model="signatureChoice">
            <SelectTrigger class="w-[140px] bg-white">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="with">Con firma</SelectItem>
                <SelectItem value="without">Sin firma</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </div>

        <Button variant="outline" @click="requestAction('pdf')" class="gap-2">
          <Download class="h-4 w-4" />
          Descargar PDF
        </Button>

        <Button @click="requestAction('print')" class="gap-2">
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
  @page {
    margin: 1.5cm;
  }
}
</style>
