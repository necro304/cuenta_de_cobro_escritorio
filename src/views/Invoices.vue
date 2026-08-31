<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import {
  Plus,
  Trash2,
  Printer,
  Pencil,
  Wallet,
  Search,
  Filter,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  Download,
  FileX2,
  FileText,
  RotateCcw,
  TrendingUp,
  X,
} from '@lucide/vue'
import { useRouter } from 'vue-router'
import PageHeader from '@/components/PageHeader.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useToast } from '@/components/ui/toast/use-toast'
import { useProfile } from '@/composables/useProfile'
import { formatCurrency, formatDate, getTodayDate } from '@/lib/format'
import { calculateBalance, determineStatus } from '@/lib/invoice'
import SignatureAskDialog from '@/components/SignatureAskDialog.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import type { Invoice, InvoicePayment } from '@/types'

const router = useRouter()
const { toast } = useToast()
const invoices = ref<Invoice[]>([])
const isLoading = ref(true)
const loadError = ref(false)
const exportingInvoiceId = ref<number | null>(null)
const pendingDeletion = ref<{ kind: 'payment' | 'invoice'; id: number } | null>(null)
const isDeleting = ref(false)

// Perfil para la descarga rápida de PDF (plantilla predeterminada y modo de firma)
const { profile, loadProfile } = useProfile()
const isAskDialogOpen = ref(false)
const pendingPdfInvoice = ref<Invoice | null>(null)

const exportPdf = async (invoice: Invoice, includeSignature: boolean) => {
  exportingInvoiceId.value = invoice.id
  try {
    const result = await window.electronAPI.exportPdf({
      invoiceId: invoice.id,
      invoiceNumber: invoice.number,
      template: profile.value.default_template ?? 'default',
      includeSignature,
    })
    if (result.success) {
      toast({ title: 'PDF descargado', description: result.message })
    } else if (result.message !== 'Operación cancelada.') {
      toast({ title: 'Error', description: result.message, variant: 'destructive' })
    }
  } finally {
    exportingInvoiceId.value = null
  }
}

const downloadPdf = (invoice: Invoice) => {
  if (profile.value.signature && profile.value.signature_mode === 'ask') {
    pendingPdfInvoice.value = invoice
    isAskDialogOpen.value = true
    return
  }
  const withSignature = profile.value.signature_mode === 'auto' && !!profile.value.signature
  exportPdf(invoice, withSignature)
}

const onSignatureChoice = (withSignature: boolean) => {
  const invoice = pendingPdfInvoice.value
  pendingPdfInvoice.value = null
  if (invoice) exportPdf(invoice, withSignature)
}

// Search, Filter and Pagination State
const searchQuery = ref('')
const statusFilter = ref('all')
const currentPage = ref(1)
const itemsPerPage = ref(10)

const filteredInvoices = computed(() => {
  return invoices.value.filter((invoice) => {
    const matchesSearch =
      invoice.client_name?.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      invoice.number.toString().includes(searchQuery.value)

    const matchesStatus = statusFilter.value === 'all' || invoice.status === statusFilter.value

    return matchesSearch && matchesStatus
  })
})

const totalPages = computed(
  () => Math.ceil(filteredInvoices.value.length / itemsPerPage.value) || 1,
)
const hasFilters = computed(() => searchQuery.value.trim() !== '' || statusFilter.value !== 'all')

const paginatedInvoices = computed(() => {
  const start = (currentPage.value - 1) * itemsPerPage.value
  const end = start + itemsPerPage.value
  return filteredInvoices.value.slice(start, end)
})

const financialSummary = computed(() => {
  const summary = invoices.value.reduce(
    (result, invoice) => {
      const paid = Math.min(Math.max(invoice.paid_amount ?? 0, 0), invoice.total)
      result.totalIssued += invoice.total
      result.totalCollected += paid
      if (invoice.status !== 'paid') result.pendingCount += 1
      return result
    },
    { totalIssued: 0, totalCollected: 0, pendingCount: 0 },
  )

  return {
    ...summary,
    balance: Math.max(summary.totalIssued - summary.totalCollected, 0),
    collectionRate:
      summary.totalIssued > 0 ? (summary.totalCollected / summary.totalIssued) * 100 : 0,
  }
})

const statusCounts = computed(() => ({
  all: invoices.value.length,
  draft: invoices.value.filter((invoice) => invoice.status === 'draft').length,
  partially_paid: invoices.value.filter((invoice) => invoice.status === 'partially_paid').length,
  paid: invoices.value.filter((invoice) => invoice.status === 'paid').length,
}))

const filteredAmount = computed(() =>
  filteredInvoices.value.reduce((total, invoice) => total + invoice.total, 0),
)

const getPaymentProgress = (invoice: Invoice) => {
  if (invoice.total <= 0) return 0
  return Math.min(Math.max(((invoice.paid_amount ?? 0) / invoice.total) * 100, 0), 100)
}

watch([searchQuery, statusFilter], () => {
  currentPage.value = 1
})

const clearFilters = () => {
  searchQuery.value = ''
  statusFilter.value = 'all'
}

// Payments Modal State
const isPaymentsModalOpen = ref(false)
const selectedInvoice = ref<Invoice | null>(null)
const payments = ref<InvoicePayment[]>([])
const newPayment = ref({
  date: getTodayDate(),
  amount: 0,
  notes: '',
})

const balancePendiente = computed(() => {
  if (!selectedInvoice.value) return 0
  const totalPagado = payments.value.reduce((sum, p) => sum + p.amount, 0)
  return calculateBalance(selectedInvoice.value.total, totalPagado)
})

const loadInvoices = async () => {
  isLoading.value = true
  loadError.value = false
  try {
    invoices.value = await window.electronAPI.dbQuery<Invoice>(`
      SELECT i.*, c.name as client_name,
      COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0) as paid_amount
      FROM invoices i
      JOIN clients c ON i.client_id = c.id
      ORDER BY i.date DESC, i.number DESC
    `)
  } catch {
    loadError.value = true
    toast({
      title: 'No se pudieron cargar las cuentas',
      description: 'Intenta consultar la base de datos nuevamente.',
      variant: 'destructive',
    })
  } finally {
    isLoading.value = false
  }
}

const openPayments = async (invoice: Invoice) => {
  selectedInvoice.value = invoice
  newPayment.value = {
    date: getTodayDate(),
    amount: calculateBalance(invoice.total, invoice.paid_amount || 0),
    notes: '',
  }
  await loadPayments(invoice.id)
  isPaymentsModalOpen.value = true
}

const loadPayments = async (invoiceId: number) => {
  payments.value = await window.electronAPI.dbQuery<InvoicePayment>(
    'SELECT * FROM invoice_payments WHERE invoice_id = ? ORDER BY date DESC, id DESC',
    [invoiceId],
  )
}

const checkAndUpdateInvoiceStatus = async (invoiceId: number) => {
  await loadInvoices() // Ensure we have latest paid_amount
  const invoice = invoices.value.find((i) => i.id === invoiceId)
  if (!invoice) return

  const newStatus = determineStatus(invoice.total, invoice.paid_amount || 0)

  if (newStatus !== invoice.status) {
    await window.electronAPI.dbRun('UPDATE invoices SET status = ? WHERE id = ?', [
      newStatus,
      invoiceId,
    ])
    await loadInvoices()
  }
}

const validatePayment = (): string | null => {
  if (newPayment.value.amount <= 0) return 'El monto debe ser mayor a 0'
  return null
}

const savePayment = async () => {
  if (!selectedInvoice.value) return
  const error = validatePayment()
  if (error) {
    toast({ title: 'No se pudo registrar el abono', description: error, variant: 'destructive' })
    return
  }

  try {
    await window.electronAPI.dbRun(
      'INSERT INTO invoice_payments (invoice_id, date, amount, notes) VALUES (?, ?, ?, ?)',
      [
        selectedInvoice.value.id,
        newPayment.value.date,
        newPayment.value.amount,
        newPayment.value.notes,
      ],
    )

    await loadPayments(selectedInvoice.value.id)
    await checkAndUpdateInvoiceStatus(selectedInvoice.value.id)

    newPayment.value.amount = balancePendiente.value
    newPayment.value.notes = ''
    toast({ title: 'Abono registrado', description: 'El pago parcial se ha guardado.' })
  } catch (e) {
    toast({ title: 'Error', description: 'No se pudo guardar el abono', variant: 'destructive' })
  }
}

const deletePayment = (id: number) => {
  pendingDeletion.value = { kind: 'payment', id }
}

const deleteInvoice = (id: number) => {
  pendingDeletion.value = { kind: 'invoice', id }
}

const closeDeletionDialog = (open: boolean) => {
  if (!open && !isDeleting.value) pendingDeletion.value = null
}

const confirmDeletion = async () => {
  const deletion = pendingDeletion.value
  if (!deletion) return

  isDeleting.value = true
  try {
    if (deletion.kind === 'payment') {
      if (!selectedInvoice.value) return
      await window.electronAPI.dbRun('DELETE FROM invoice_payments WHERE id = ?', [deletion.id])
      await loadPayments(selectedInvoice.value.id)
      await checkAndUpdateInvoiceStatus(selectedInvoice.value.id)
      toast({ title: 'Abono eliminado' })
    } else {
      await window.electronAPI.dbRun('DELETE FROM invoices WHERE id = ?', [deletion.id])
      await loadInvoices()
      toast({ title: 'Cuenta eliminada' })
    }
    pendingDeletion.value = null
  } catch {
    toast({
      title: 'No se pudo eliminar el registro',
      description: 'La base de datos no respondió correctamente.',
      variant: 'destructive',
    })
  } finally {
    isDeleting.value = false
  }
}

const handleNewInvoice = () => {
  router.push('/invoices/new')
}

onMounted(() => {
  loadInvoices()
  loadProfile()
})
</script>

<template>
  <div class="app-page">
    <PageHeader
      title="Libro de cuentas"
      description="Controla lo emitido, lo recaudado y cada saldo pendiente desde una sola vista."
    >
      <template #leading>
        <p class="font-mono text-[10px] font-semibold tracking-[0.16em] text-primary">
          CONTROL DE CARTERA · {{ statusCounts.all }} REGISTROS
        </p>
      </template>
      <template #actions>
        <Button class="hidden md:inline-flex" @click="handleNewInvoice">
          <Plus /> Nueva cuenta
        </Button>
      </template>
    </PageHeader>

    <section
      v-if="!loadError"
      class="invoice-overview overflow-hidden rounded-[1.5rem] border border-primary/15 bg-card shadow-[0_24px_70px_hsl(var(--primary)/0.08)]"
    >
      <div class="grid xl:grid-cols-12">
        <div class="relative isolate overflow-hidden bg-primary p-6 text-primary-foreground sm:p-8 xl:col-span-5">
          <div class="overview-orbit" aria-hidden="true"></div>
          <div class="relative z-10 flex min-h-48 flex-col justify-between gap-10">
            <div class="flex items-center justify-between gap-4">
              <p class="flex items-center gap-2 text-sm font-medium text-primary-foreground/70">
                <Wallet class="size-4" :stroke-width="1.8" /> Saldo por cobrar
              </p>
              <span class="rounded-md border border-primary-foreground/15 bg-primary-foreground/10 px-2 py-1 font-mono text-[10px] font-semibold tracking-[0.12em]">
                COP
              </span>
            </div>
            <div>
              <p class="metric-value text-[clamp(2.25rem,4vw,3.75rem)] font-semibold leading-none">
                ${{ formatCurrency(financialSummary.balance) }}
              </p>
              <div class="mt-5 flex items-center gap-3">
                <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-primary-foreground/15">
                  <div
                    class="h-full rounded-full bg-primary-foreground transition-[width] duration-700"
                    :style="{ width: `${financialSummary.collectionRate}%` }"
                  ></div>
                </div>
                <span class="metric-value text-xs font-semibold">
                  {{ financialSummary.collectionRate.toFixed(1) }}%
                </span>
              </div>
              <p class="mt-2 text-xs text-primary-foreground/60">avance de recaudo histórico</p>
            </div>
          </div>
        </div>

        <div class="grid sm:grid-cols-3 xl:col-span-7">
          <div class="flex min-h-36 flex-col justify-between border-b p-5 sm:border-b-0 sm:border-r sm:p-6">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Valor emitido</p>
              <FileText class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <div>
              <p class="metric-value text-2xl font-semibold">${{ formatCurrency(financialSummary.totalIssued) }}</p>
              <p class="mt-2 text-xs text-muted-foreground">en {{ statusCounts.all }} cuentas</p>
            </div>
          </div>
          <div class="flex min-h-36 flex-col justify-between border-b p-5 sm:border-b-0 sm:border-r sm:p-6">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Recaudado</p>
              <TrendingUp class="size-5 text-[hsl(var(--success))]" :stroke-width="1.8" />
            </div>
            <div>
              <p class="metric-value text-2xl font-semibold">${{ formatCurrency(financialSummary.totalCollected) }}</p>
              <p class="mt-2 text-xs text-muted-foreground">abonos acumulados</p>
            </div>
          </div>
          <div class="flex min-h-36 flex-col justify-between p-5 sm:p-6">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Por gestionar</p>
              <Filter class="size-5 text-[hsl(var(--warning))]" :stroke-width="1.8" />
            </div>
            <div>
              <p class="metric-value text-3xl font-semibold">{{ financialSummary.pendingCount }}</p>
              <p class="mt-2 text-xs text-muted-foreground">sin recaudo completo</p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section v-if="!loadError" class="surface overflow-hidden">
      <div class="flex flex-col gap-3 border-b p-3 xl:flex-row xl:items-center">
        <label class="relative min-w-0 flex-1">
          <span class="sr-only">Buscar cuentas</span>
          <Search class="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <input v-model="searchQuery" type="search" placeholder="Buscar cliente o número de cuenta" class="form-control border-0 bg-secondary/65 pl-10 pr-10 shadow-none focus:bg-background" />
          <button
            v-if="searchQuery"
            type="button"
            class="absolute right-2 top-1/2 flex size-8 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-background hover:text-foreground"
            aria-label="Limpiar búsqueda"
            @click="searchQuery = ''"
          >
            <X class="size-4" />
          </button>
        </label>

        <div class="hidden items-center gap-1 rounded-lg bg-secondary/65 p-1 xl:flex" aria-label="Filtrar por estado">
          <button
            v-for="option in [
              { value: 'all', label: 'Todas', count: statusCounts.all },
              { value: 'draft', label: 'Pendientes', count: statusCounts.draft },
              { value: 'partially_paid', label: 'Con abonos', count: statusCounts.partially_paid },
              { value: 'paid', label: 'Pagadas', count: statusCounts.paid },
            ]"
            :key="option.value"
            type="button"
            class="flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium transition-[background-color,color,box-shadow]"
            :class="statusFilter === option.value ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
            @click="statusFilter = option.value"
          >
            {{ option.label }}
            <span class="font-mono text-[10px] opacity-65">{{ option.count }}</span>
          </button>
        </div>

        <label class="relative xl:hidden">
          <span class="sr-only">Filtrar por estado</span>
          <Filter class="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <select v-model="statusFilter" class="form-control cursor-pointer pl-10">
            <option value="all">Todas ({{ statusCounts.all }})</option>
            <option value="draft">Pendientes ({{ statusCounts.draft }})</option>
            <option value="partially_paid">Con abonos ({{ statusCounts.partially_paid }})</option>
            <option value="paid">Pagadas ({{ statusCounts.paid }})</option>
          </select>
        </label>
      </div>
      <div class="flex flex-wrap items-center justify-between gap-2 bg-secondary/25 px-4 py-3 text-xs text-muted-foreground">
        <p><span class="font-semibold text-foreground">{{ filteredInvoices.length }}</span> resultados</p>
        <p class="metric-value">Valor filtrado: <span class="font-semibold text-foreground">${{ formatCurrency(filteredAmount) }}</span></p>
      </div>
    </section>

    <div v-if="loadError" class="surface empty-state">
      <div class="empty-state-icon"><RotateCcw class="size-5" /></div>
      <h2 class="section-title">No pudimos cargar las cuentas</h2>
      <p class="mt-2 max-w-md text-sm text-muted-foreground">
        La base de datos no respondió correctamente.
      </p>
      <Button class="mt-5" variant="outline" @click="loadInvoices"><RotateCcw /> Reintentar</Button>
    </div>

    <template v-else>
      <div class="surface hidden overflow-hidden md:block">
        <div class="flex items-center justify-between border-b px-5 py-4">
          <div>
            <h2 class="section-title">Registro de cuentas</h2>
            <p class="mt-1 text-xs text-muted-foreground">Ordenadas por fecha de emisión más reciente.</p>
          </div>
          <p class="font-mono text-[10px] font-semibold tracking-[0.12em] text-muted-foreground">COP · CARTERA</p>
        </div>
        <div class="overflow-x-auto">
        <table class="data-table min-w-[980px]">
          <thead>
            <tr>
              <th class="w-36">Documento</th>
              <th>Cliente</th>
              <th class="w-64">Avance de pago</th>
              <th class="w-36 text-right">Saldo</th>
              <th>Estado</th>
              <th class="w-60 text-right">Acciones</th>
            </tr>
          </thead>
          <tbody>
            <template v-if="isLoading">
              <tr v-for="row in 5" :key="row" aria-hidden="true">
                <td v-for="column in 6" :key="column">
                  <div class="h-5 animate-pulse rounded bg-secondary"></div>
                </td>
              </tr>
            </template>
            <tr v-for="invoice in isLoading ? [] : paginatedInvoices" :key="invoice.id" class="group">
              <td>
                <p class="font-mono text-sm font-semibold text-primary">#{{ invoice.number }}</p>
                <p class="mt-1 text-xs text-muted-foreground">{{ formatDate(invoice.date) }}</p>
              </td>
              <td>
                <p class="max-w-64 truncate font-semibold">{{ invoice.client_name }}</p>
                <p class="mt-1 text-xs text-muted-foreground">Cuenta de cobro</p>
              </td>
              <td>
                <div class="flex items-end justify-between gap-3">
                  <div>
                    <p class="metric-value font-semibold">${{ formatCurrency(invoice.total) }}</p>
                    <p class="mt-1 text-[11px] text-muted-foreground">
                      Recaudado ${{ formatCurrency(invoice.paid_amount || 0) }}
                    </p>
                  </div>
                  <span class="metric-value text-[11px] font-semibold text-muted-foreground">
                    {{ getPaymentProgress(invoice).toFixed(0) }}%
                  </span>
                </div>
                <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-secondary">
                  <div
                    class="h-full rounded-full transition-[width] duration-500"
                    :class="invoice.status === 'paid' ? 'bg-[hsl(var(--success))]' : 'bg-primary'"
                    :style="{ width: `${getPaymentProgress(invoice)}%` }"
                  ></div>
                </div>
              </td>
              <td class="text-right">
                <p class="metric-value font-semibold" :class="invoice.status === 'paid' ? 'text-[hsl(var(--success))]' : ''">
                  ${{ formatCurrency(calculateBalance(invoice.total, invoice.paid_amount || 0)) }}
                </p>
                <p class="mt-1 text-[11px] text-muted-foreground">
                  {{ invoice.status === 'paid' ? 'Saldada' : 'Pendiente' }}
                </p>
              </td>
              <td><StatusBadge :status="invoice.status" /></td>
              <td>
                <div class="flex items-center justify-end gap-1.5">
                  <button
                    class="icon-button border-primary/20 bg-accent/55 text-primary hover:bg-accent"
                    type="button"
                    :aria-label="`Gestionar abonos de la cuenta ${invoice.number}`"
                    title="Abonos"
                    @click="openPayments(invoice)"
                  >
                    <Wallet />
                  </button>
                  <button
                    class="icon-button"
                    type="button"
                    :aria-label="`Editar cuenta ${invoice.number}`"
                    title="Editar"
                    @click="router.push('/invoices/edit/' + invoice.id)"
                  >
                    <Pencil />
                  </button>
                  <button
                    class="icon-button"
                    type="button"
                    :aria-label="`Ver e imprimir la cuenta ${invoice.number}`"
                    title="Imprimir"
                    @click="router.push('/print/' + invoice.id)"
                  >
                    <Printer />
                  </button>
                  <button
                    class="icon-button"
                    type="button"
                    :aria-label="`Descargar PDF de la cuenta ${invoice.number}`"
                    title="Descargar PDF"
                    :disabled="exportingInvoiceId === invoice.id"
                    @click="downloadPdf(invoice)"
                  >
                    <Download />
                  </button>
                  <button
                    class="icon-button hover:border-destructive/30 hover:bg-destructive/10 hover:text-destructive"
                    type="button"
                    :aria-label="`Eliminar cuenta ${invoice.number}`"
                    title="Eliminar"
                    @click="deleteInvoice(invoice.id)"
                  >
                    <Trash2 />
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="!isLoading && filteredInvoices.length === 0">
              <td colspan="6" class="p-0">
                <div class="empty-state">
                  <div class="empty-state-icon"><FileX2 class="size-5" /></div>
                  <h2 class="section-title">
                    {{ hasFilters ? 'No hay coincidencias' : 'Aún no tienes cuentas' }}
                  </h2>
                  <p class="mt-2 max-w-md text-sm text-muted-foreground">
                    {{
                      hasFilters
                        ? 'Prueba otra búsqueda o limpia los filtros.'
                        : 'Crea tu primera cuenta de cobro para comenzar.'
                    }}
                  </p>
                  <Button v-if="hasFilters" class="mt-5" variant="outline" @click="clearFilters">
                    Limpiar filtros
                  </Button>
                  <Button v-else class="mt-5" @click="handleNewInvoice"
                    ><Plus /> Nueva cuenta</Button
                  >
                </div>
              </td>
            </tr>
          </tbody>
        </table>
        </div>
      </div>

      <div class="space-y-3 md:hidden" :aria-busy="isLoading">
        <template v-if="isLoading">
          <div
            v-for="row in 4"
            :key="row"
            class="surface h-48 animate-pulse bg-secondary/70"
            aria-hidden="true"
          ></div>
        </template>

        <article
          v-for="invoice in isLoading ? [] : paginatedInvoices"
          :key="invoice.id"
          class="surface relative overflow-hidden"
        >
          <div
            class="absolute inset-x-0 top-0 h-1 bg-primary transition-[width]"
            :style="{ width: `${getPaymentProgress(invoice)}%` }"
            aria-hidden="true"
          ></div>
          <div class="flex items-start justify-between gap-3 p-5 pb-4">
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <p class="font-mono text-sm font-semibold text-primary">#{{ invoice.number }}</p>
                <span class="text-xs text-muted-foreground">{{ formatDate(invoice.date) }}</span>
              </div>
              <h2 class="mt-2 truncate text-lg font-semibold tracking-[-0.025em]">{{ invoice.client_name }}</h2>
            </div>
            <StatusBadge :status="invoice.status" />
          </div>

          <div class="mx-4 grid grid-cols-2 gap-4 rounded-xl bg-secondary/60 p-4">
            <div>
              <p class="text-[11px] font-medium text-muted-foreground">VALOR EMITIDO</p>
              <p class="metric-value mt-1.5 text-lg font-semibold">
                ${{ formatCurrency(invoice.total) }}
              </p>
              <p class="mt-1 text-[11px] text-muted-foreground">
                Recaudado ${{ formatCurrency(invoice.paid_amount || 0) }}
              </p>
            </div>
            <div class="border-l pl-4 text-right">
              <p class="text-[11px] font-medium text-muted-foreground">SALDO</p>
              <p class="metric-value mt-1.5 text-lg font-semibold" :class="invoice.status === 'paid' ? 'text-[hsl(var(--success))]' : ''">
                ${{ formatCurrency(calculateBalance(invoice.total, invoice.paid_amount || 0)) }}
              </p>
              <p class="metric-value mt-1 text-[11px] text-muted-foreground">
                {{ getPaymentProgress(invoice).toFixed(0) }}% pagado
              </p>
            </div>
          </div>

          <div class="mt-4 grid grid-cols-5 border-t border-border/70 bg-secondary/20 p-2">
            <button
              type="button"
              class="flex min-h-12 flex-col items-center justify-center gap-1 rounded-lg bg-accent/60 text-[10px] font-semibold text-primary transition-colors hover:bg-accent"
              :aria-label="`Gestionar abonos de la cuenta ${invoice.number}`"
              @click="openPayments(invoice)"
            >
              <Wallet class="size-4" /> Abonos
            </button>
            <button
              type="button"
              class="flex min-h-12 flex-col items-center justify-center gap-1 rounded-lg text-[10px] font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
              :aria-label="`Editar cuenta ${invoice.number}`"
              @click="router.push('/invoices/edit/' + invoice.id)"
            >
              <Pencil class="size-4" /> Editar
            </button>
            <button
              type="button"
              class="flex min-h-12 flex-col items-center justify-center gap-1 rounded-lg text-[10px] font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
              :aria-label="`Ver e imprimir la cuenta ${invoice.number}`"
              @click="router.push('/print/' + invoice.id)"
            >
              <Printer class="size-4" /> Imprimir
            </button>
            <button
              type="button"
              class="flex min-h-12 flex-col items-center justify-center gap-1 rounded-lg text-[10px] font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground disabled:opacity-40"
              :aria-label="`Descargar PDF de la cuenta ${invoice.number}`"
              :disabled="exportingInvoiceId === invoice.id"
              @click="downloadPdf(invoice)"
            >
              <Download class="size-4" /> PDF
            </button>
            <button
              type="button"
              class="flex min-h-12 flex-col items-center justify-center gap-1 rounded-lg text-[10px] font-medium text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
              :aria-label="`Eliminar cuenta ${invoice.number}`"
              @click="deleteInvoice(invoice.id)"
            >
              <Trash2 class="size-4" /> Eliminar
            </button>
          </div>
        </article>

        <div v-if="!isLoading && filteredInvoices.length === 0" class="surface empty-state">
          <div class="empty-state-icon"><FileX2 class="size-5" /></div>
          <h2 class="section-title">
            {{ hasFilters ? 'No hay coincidencias' : 'Aún no tienes cuentas' }}
          </h2>
          <p class="mt-2 max-w-md text-sm text-muted-foreground">
            {{
              hasFilters
                ? 'Prueba otra búsqueda o limpia los filtros.'
                : 'Crea tu primera cuenta de cobro para comenzar.'
            }}
          </p>
          <Button v-if="hasFilters" class="mt-5" variant="outline" @click="clearFilters">
            Limpiar filtros
          </Button>
          <Button v-else class="mt-5" @click="handleNewInvoice"><Plus /> Nueva cuenta</Button>
        </div>
      </div>
    </template>

    <div
      v-if="!loadError && totalPages > 1"
      class="surface flex flex-col gap-3 px-4 py-3 text-sm sm:flex-row sm:items-center sm:justify-between"
    >
      <p class="text-xs text-muted-foreground">
        {{ (currentPage - 1) * itemsPerPage + 1 }}-{{
          Math.min(currentPage * itemsPerPage, filteredInvoices.length)
        }}
        de {{ filteredInvoices.length }}
      </p>
      <div class="flex items-center gap-1.5">
        <button
          class="icon-button"
          :disabled="currentPage === 1"
          aria-label="Primera página"
          @click="currentPage = 1"
        >
          <ChevronsLeft />
        </button>
        <button
          class="icon-button"
          :disabled="currentPage === 1"
          aria-label="Página anterior"
          @click="currentPage--"
        >
          <ChevronLeft />
        </button>
        <span class="min-w-20 text-center font-mono text-xs text-muted-foreground"
          >{{ currentPage }} de {{ totalPages }}</span
        >
        <button
          class="icon-button"
          :disabled="currentPage === totalPages"
          aria-label="Página siguiente"
          @click="currentPage++"
        >
          <ChevronRight />
        </button>
        <button
          class="icon-button"
          :disabled="currentPage === totalPages"
          aria-label="Última página"
          @click="currentPage = totalPages"
        >
          <ChevronsRight />
        </button>
      </div>
    </div>

    <Dialog v-model:open="isPaymentsModalOpen">
      <DialogContent class="sm:max-w-[620px]">
        <DialogHeader class="text-left">
          <DialogTitle class="text-2xl"
            >Abonos de la cuenta #{{ selectedInvoice?.number }}</DialogTitle
          >
          <DialogDescription>{{ selectedInvoice?.client_name }}</DialogDescription>
        </DialogHeader>

        <div class="space-y-5 text-sm">
          <div class="grid grid-cols-3 gap-3 rounded-xl bg-secondary/70 p-4">
            <div>
              <p class="text-xs text-muted-foreground">Total</p>
              <p class="metric-value mt-1 font-semibold">
                ${{ formatCurrency(selectedInvoice?.total) }}
              </p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">Pagado</p>
              <p class="metric-value mt-1 font-semibold text-[hsl(var(--success))]">
                ${{ formatCurrency((selectedInvoice?.total ?? 0) - balancePendiente) }}
              </p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">Saldo</p>
              <p class="metric-value mt-1 font-semibold">${{ formatCurrency(balancePendiente) }}</p>
            </div>
          </div>

          <div v-if="balancePendiente > 0" class="space-y-4 rounded-xl border p-4">
            <h3 class="section-title">Registrar abono</h3>
            <div class="grid gap-4 sm:grid-cols-2">
              <div class="space-y-2">
                <label for="payment-date" class="field-label">Fecha</label>
                <input
                  id="payment-date"
                  v-model="newPayment.date"
                  type="date"
                  class="form-control"
                />
              </div>
              <div class="space-y-2">
                <label for="payment-amount" class="field-label">Monto</label>
                <input
                  id="payment-amount"
                  v-model.number="newPayment.amount"
                  type="number"
                  min="1"
                  class="form-control"
                />
              </div>
            </div>
            <div class="space-y-2">
              <label for="payment-notes" class="field-label">Notas o referencia</label>
              <input
                id="payment-notes"
                v-model="newPayment.notes"
                class="form-control"
                placeholder="Ej. Transferencia 12345"
              />
            </div>
            <Button class="w-full" @click="savePayment">Guardar abono</Button>
          </div>

          <div class="space-y-3">
            <h3 class="section-title">Historial</h3>
            <div
              v-if="payments.length === 0"
              class="rounded-xl bg-secondary/60 py-7 text-center text-sm text-muted-foreground"
            >
              No hay abonos registrados.
            </div>
            <div v-else class="max-h-[220px] space-y-2 overflow-y-auto pr-1">
              <div
                v-for="payment in payments"
                :key="payment.id"
                class="flex items-center justify-between rounded-xl border p-3"
              >
                <div>
                  <p class="metric-value font-semibold">${{ formatCurrency(payment.amount) }}</p>
                  <p class="mt-0.5 text-xs text-muted-foreground">
                    {{ formatDate(payment.date) }}
                    <span v-if="payment.notes">· {{ payment.notes }}</span>
                  </p>
                </div>
                <button
                  type="button"
                  @click="deletePayment(payment.id)"
                  class="icon-button hover:bg-destructive/10 hover:text-destructive"
                  aria-label="Eliminar abono"
                  title="Eliminar abono"
                >
                  <Trash2 />
                </button>
              </div>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" @click="isPaymentsModalOpen = false">Cerrar</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <SignatureAskDialog v-model:open="isAskDialogOpen" @choice="onSignatureChoice" />
    <ConfirmDialog
      :open="pendingDeletion !== null"
      :title="pendingDeletion?.kind === 'payment' ? 'Eliminar abono' : 'Eliminar cuenta de cobro'"
      :description="
        pendingDeletion?.kind === 'payment'
          ? 'El saldo y el estado de la cuenta se recalcularán. Esta acción no se puede deshacer.'
          : 'Se eliminarán el documento, sus conceptos y sus abonos. Esta acción no se puede deshacer.'
      "
      :confirm-label="pendingDeletion?.kind === 'payment' ? 'Eliminar abono' : 'Eliminar cuenta'"
      :busy="isDeleting"
      destructive
      @update:open="closeDeletionDialog"
      @confirm="confirmDeletion"
    />
  </div>
</template>

<style scoped>
.invoice-overview {
  animation: ledger-enter 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
}

.overview-orbit {
  position: absolute;
  width: 19rem;
  height: 19rem;
  border: 1px solid hsl(var(--primary-foreground) / 0.1);
  border-radius: 999px;
  top: -11rem;
  right: -6rem;
  box-shadow:
    0 0 0 2.5rem hsl(var(--primary-foreground) / 0.035),
    0 0 0 6rem hsl(var(--primary-foreground) / 0.02);
}

@keyframes ledger-enter {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
