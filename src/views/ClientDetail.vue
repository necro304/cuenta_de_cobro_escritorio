<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import {
  ArrowLeft,
  ArrowRight,
  Banknote,
  CalendarDays,
  Download,
  FileText,
  Mail,
  MapPin,
  Pencil,
  Phone,
  Plus,
  Printer,
  ReceiptText,
  RotateCcw,
  WalletCards,
} from '@lucide/vue'
import { Bar } from 'vue-chartjs'
import { BarElement, CategoryScale, Chart as ChartJS, Legend, LinearScale, Tooltip } from 'chart.js'
import type { TooltipItem } from 'chart.js'
import PageHeader from '@/components/PageHeader.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/toast/use-toast'
import { useTheme } from '@/composables/useTheme'
import { formatCurrency, formatDate } from '@/lib/format'
import type { Client, Invoice, InvoicePayment } from '@/types'

ChartJS.register(Tooltip, Legend, BarElement, CategoryScale, LinearScale)

interface MonthlyAmount {
  month: string
  total: number
}

interface ClientPayment extends InvoicePayment {
  invoice_number: number
}

const route = useRoute()
const router = useRouter()
const { toast } = useToast()
const { resolvedTheme } = useTheme()

const client = ref<Client | null>(null)
const invoices = ref<Invoice[]>([])
const monthlyInvoices = ref<MonthlyAmount[]>([])
const monthlyPayments = ref<MonthlyAmount[]>([])
const recentPayments = ref<ClientPayment[]>([])
const isLoading = ref(true)
const isExportingReport = ref(false)
const loadError = ref(false)
const notFound = ref(false)

const readChartColors = () => {
  const styles = getComputedStyle(document.documentElement)
  const hsl = (name: string) => `hsl(${styles.getPropertyValue(name).trim()})`
  return {
    foreground: hsl('--foreground'),
    mutedForeground: hsl('--muted-foreground'),
    border: hsl('--border'),
    primary: hsl('--primary'),
    success: hsl('--success'),
  }
}

const chartColors = ref(readChartColors())

const financialSummary = computed(() => {
  const summary = invoices.value.reduce(
    (result, invoice) => {
      const paid = Math.min(Math.max(invoice.paid_amount ?? 0, 0), invoice.total)
      result.issued += invoice.total
      result.collected += paid
      if (invoice.status !== 'paid') result.pendingCount += 1
      return result
    },
    { issued: 0, collected: 0, pendingCount: 0 },
  )

  return {
    ...summary,
    balance: Math.max(summary.issued - summary.collected, 0),
    average: invoices.value.length > 0 ? summary.issued / invoices.value.length : 0,
    collectionRate: summary.issued > 0 ? (summary.collected / summary.issued) * 100 : 0,
  }
})

const getInitials = (name: string) =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase())
    .join('')

const lastActivityDate = computed(() => {
  const dates = [
    ...invoices.value.map((invoice) => invoice.date),
    ...recentPayments.value.map((payment) => payment.date),
  ].sort((left, right) => right.localeCompare(left))
  return dates[0]
})

const relationshipSince = computed(() => {
  const createdAt = client.value?.created_at
  if (!createdAt) return 'Fecha no disponible'
  const parsedDate = new Date(createdAt.replace(' ', 'T'))
  if (Number.isNaN(parsedDate.getTime())) return formatDate(createdAt.slice(0, 10))
  return new Intl.DateTimeFormat('es-CO', { month: 'long', year: 'numeric' }).format(parsedDate)
})

const statusMeta: Record<
  Invoice['status'],
  { label: string; description: string; dotClass: string; barClass: string }
> = {
  paid: {
    label: 'Pagadas',
    description: 'Recaudadas por completo',
    dotClass: 'bg-[hsl(var(--success))]',
    barClass: 'bg-[hsl(var(--success))]',
  },
  partially_paid: {
    label: 'Con abonos',
    description: 'Tienen pagos parciales',
    dotClass: 'bg-[hsl(var(--info))]',
    barClass: 'bg-[hsl(var(--info))]',
  },
  draft: {
    label: 'Pendientes',
    description: 'Sin recaudo completo',
    dotClass: 'bg-[hsl(var(--warning))]',
    barClass: 'bg-[hsl(var(--warning))]',
  },
}

const statusRows = computed(() =>
  (['paid', 'partially_paid', 'draft'] as const).map((status) => {
    const matchingInvoices = invoices.value.filter((invoice) => invoice.status === status)
    return {
      status,
      count: matchingInvoices.length,
      total: matchingInvoices.reduce((total, invoice) => total + invoice.total, 0),
      percentage:
        invoices.value.length > 0 ? (matchingInvoices.length / invoices.value.length) * 100 : 0,
      ...statusMeta[status],
    }
  }),
)

const lastSixMonths = computed(() => {
  const now = new Date()
  return Array.from({ length: 6 }, (_, index) => {
    const date = new Date(now.getFullYear(), now.getMonth() - 5 + index, 1)
    return {
      month: `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`,
      label: new Intl.DateTimeFormat('es-CO', { month: 'short' }).format(date).replace('.', ''),
    }
  })
})

const barChartData = computed(() => {
  const issuedByMonth = new Map(monthlyInvoices.value.map((row) => [row.month, row.total]))
  const collectedByMonth = new Map(monthlyPayments.value.map((row) => [row.month, row.total]))
  return {
    labels: lastSixMonths.value.map((item) => item.label),
    datasets: [
      {
        label: 'Emitido',
        backgroundColor: chartColors.value.primary,
        borderRadius: 5,
        borderSkipped: false,
        data: lastSixMonths.value.map((item) => issuedByMonth.get(item.month) ?? 0),
      },
      {
        label: 'Recaudado',
        backgroundColor: chartColors.value.success,
        borderRadius: 5,
        borderSkipped: false,
        data: lastSixMonths.value.map((item) => collectedByMonth.get(item.month) ?? 0),
      },
    ],
  }
})

const barOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  interaction: { mode: 'index' as const, intersect: false },
  scales: {
    y: {
      beginAtZero: true,
      border: { display: false },
      grid: { color: chartColors.value.border },
      ticks: {
        callback: (value: string | number) =>
          `$${Intl.NumberFormat('es-CO', { notation: 'compact' }).format(Number(value))}`,
        color: chartColors.value.mutedForeground,
        font: { family: 'Outfit Variable', weight: 500 },
        maxTicksLimit: 5,
      },
    },
    x: {
      border: { display: false },
      grid: { display: false },
      ticks: {
        color: chartColors.value.mutedForeground,
        font: { family: 'Outfit Variable', weight: 500 },
      },
    },
  },
  plugins: {
    legend: {
      align: 'end' as const,
      labels: {
        boxHeight: 8,
        boxWidth: 8,
        color: chartColors.value.foreground,
        font: { family: 'Outfit Variable', weight: 500 },
        padding: 18,
        usePointStyle: true,
      },
    },
    tooltip: {
      callbacks: {
        label: (context: TooltipItem<'bar'>) =>
          ` ${context.dataset.label}: $${formatCurrency(Number(context.parsed.y))}`,
      },
    },
  },
}))

const chartSummary = computed(() =>
  lastSixMonths.value
    .map((item, index) => {
      const issued = barChartData.value.datasets[0].data[index]
      const collected = barChartData.value.datasets[1].data[index]
      return `${item.label}: emitido $${formatCurrency(issued)}, recaudado $${formatCurrency(collected)}`
    })
    .join('. '),
)

const loadClient = async () => {
  const clientId = Number(route.params.id)
  if (!Number.isSafeInteger(clientId) || clientId <= 0) {
    notFound.value = true
    isLoading.value = false
    return
  }

  isLoading.value = true
  loadError.value = false
  notFound.value = false

  try {
    const [clientRecord, clientInvoices, issuedByMonth, collectedByMonth, payments] =
      await Promise.all([
        window.electronAPI.dbGet<Client>('SELECT * FROM clients WHERE id = ?', [clientId]),
        window.electronAPI.dbQuery<Invoice>(
          `SELECT i.*,
            COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0) as paid_amount,
            MAX(i.total - COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0), 0) as balance
          FROM invoices i
          WHERE i.client_id = ?
          ORDER BY i.date DESC, i.number DESC`,
          [clientId],
        ),
        window.electronAPI.dbQuery<MonthlyAmount>(
          `SELECT strftime('%Y-%m', date) as month, COALESCE(SUM(total), 0) as total
          FROM invoices
          WHERE client_id = ?
            AND date >= date('now', 'localtime', 'start of month', '-5 months')
          GROUP BY month
          ORDER BY month`,
          [clientId],
        ),
        window.electronAPI.dbQuery<MonthlyAmount>(
          `SELECT strftime('%Y-%m', p.date) as month, COALESCE(SUM(p.amount), 0) as total
          FROM invoice_payments p
          JOIN invoices i ON i.id = p.invoice_id
          WHERE i.client_id = ?
            AND p.date >= date('now', 'localtime', 'start of month', '-5 months')
          GROUP BY month
          ORDER BY month`,
          [clientId],
        ),
        window.electronAPI.dbQuery<ClientPayment>(
          `SELECT p.*, i.number as invoice_number
          FROM invoice_payments p
          JOIN invoices i ON i.id = p.invoice_id
          WHERE i.client_id = ?
          ORDER BY p.date DESC, p.id DESC
          LIMIT 8`,
          [clientId],
        ),
      ])

    if (!clientRecord) {
      notFound.value = true
      return
    }

    client.value = clientRecord
    invoices.value = clientInvoices
    monthlyInvoices.value = issuedByMonth
    monthlyPayments.value = collectedByMonth
    recentPayments.value = payments
  } catch {
    loadError.value = true
    toast({
      title: 'No se pudo cargar el cliente',
      description: 'Intenta consultar la base de datos nuevamente.',
      variant: 'destructive',
    })
  } finally {
    isLoading.value = false
  }
}

const exportClientReport = async () => {
  if (!client.value) return
  isExportingReport.value = true
  try {
    const result = await window.electronAPI.exportClientReportPdf({
      clientId: client.value.id,
      clientName: client.value.name,
    })
    if (result.success) {
      toast({ title: 'Informe descargado', description: result.message })
    } else if (result.message !== 'Operación cancelada.') {
      toast({
        title: 'No se pudo generar el informe',
        description: result.message,
        variant: 'destructive',
      })
    }
  } finally {
    isExportingReport.value = false
  }
}

onMounted(() => {
  chartColors.value = readChartColors()
  loadClient()
})

watch(resolvedTheme, async () => {
  await nextTick()
  chartColors.value = readChartColors()
})
</script>

<template>
  <div class="app-page">
    <PageHeader
      :title="client?.name ?? 'Detalle del cliente'"
      :description="
        client
          ? 'Historial financiero, recaudo y datos de contacto en una sola vista.'
          : 'Consulta la relación comercial y sus movimientos.'
      "
    >
      <template #leading>
        <RouterLink
          to="/clients"
          class="group inline-flex items-center gap-2 text-sm font-medium text-muted-foreground transition-colors hover:text-primary"
        >
          <ArrowLeft class="size-4 transition-transform group-hover:-translate-x-0.5" />
          Volver al directorio
        </RouterLink>
      </template>
      <template v-if="client" #actions>
        <Button variant="outline" :disabled="isExportingReport" @click="exportClientReport">
          <Download class="size-4" />
          {{ isExportingReport ? 'Generando…' : 'Informe PDF' }}
        </Button>
        <Button @click="router.push(`/invoices/new?clientId=${client.id}`)">
          <Plus class="size-4" /> Nueva cuenta
        </Button>
      </template>
    </PageHeader>

    <div v-if="isLoading" class="grid animate-pulse gap-4 xl:grid-cols-12" aria-busy="true">
      <div class="h-[350px] rounded-[1.5rem] bg-secondary xl:col-span-7"></div>
      <div class="h-[350px] rounded-[1.125rem] bg-secondary xl:col-span-5"></div>
      <div class="h-36 rounded-[1.125rem] bg-secondary xl:col-span-12"></div>
      <div class="h-[390px] rounded-[1.125rem] bg-secondary xl:col-span-8"></div>
      <div class="h-[390px] rounded-[1.125rem] bg-secondary xl:col-span-4"></div>
    </div>

    <section v-else-if="loadError" class="surface empty-state min-h-[360px]">
      <div class="empty-state-icon"><RotateCcw class="size-5" /></div>
      <h2 class="section-title">No pudimos cargar este cliente</h2>
      <p class="mt-2 max-w-md text-sm text-muted-foreground">
        La base de datos no respondió correctamente.
      </p>
      <Button class="mt-5" variant="outline" @click="loadClient"><RotateCcw /> Reintentar</Button>
    </section>

    <section v-else-if="notFound" class="surface empty-state min-h-[360px]">
      <div class="empty-state-icon"><FileText class="size-5" /></div>
      <p class="mb-3 font-mono text-xs font-semibold tracking-[0.14em] text-primary">
        REGISTRO NO ENCONTRADO
      </p>
      <h2 class="text-2xl font-semibold tracking-[-0.04em]">Este cliente no está disponible</h2>
      <p class="mt-3 max-w-md text-sm leading-6 text-muted-foreground">
        El registro pudo eliminarse o la dirección no corresponde a un cliente válido.
      </p>
      <Button class="mt-6" variant="outline" @click="router.push('/clients')">
        <ArrowLeft /> Volver al directorio
      </Button>
    </section>

    <template v-else-if="client">
      <div class="grid gap-4 xl:grid-cols-12">
        <section
          class="client-hero relative isolate overflow-hidden rounded-[1.5rem] bg-primary p-6 text-primary-foreground sm:p-8 xl:col-span-7"
        >
          <div class="client-orbit" aria-hidden="true"></div>
          <div class="relative z-10 flex min-h-[286px] flex-col justify-between gap-10">
            <div class="flex items-start justify-between gap-4">
              <div class="flex min-w-0 items-center gap-4">
                <div
                  class="flex size-14 shrink-0 items-center justify-center rounded-2xl border border-primary-foreground/15 bg-primary-foreground/10 font-mono text-sm font-semibold"
                >
                  {{ getInitials(client.name) }}
                </div>
                <div class="min-w-0">
                  <p class="font-medium">{{ client.name }}</p>
                  <p class="mt-1 font-mono text-xs text-primary-foreground/60">
                    {{ client.document_id || 'Documento sin registrar' }}
                  </p>
                </div>
              </div>
              <span
                class="shrink-0 rounded-lg border border-primary-foreground/15 bg-primary-foreground/10 px-2.5 py-1 font-mono text-[10px] font-semibold tracking-[0.12em]"
              >
                COP
              </span>
            </div>

            <div>
              <p class="text-sm text-primary-foreground/60">Cartera por cobrar</p>
              <p
                class="metric-value mt-3 text-[clamp(2.3rem,5vw,4.25rem)] font-semibold leading-none"
              >
                ${{ formatCurrency(financialSummary.balance) }}
              </p>
              <p class="mt-3 text-sm text-primary-foreground/60">
                {{ financialSummary.pendingCount }}
                {{
                  financialSummary.pendingCount === 1 ? 'cuenta pendiente' : 'cuentas pendientes'
                }}
                de {{ invoices.length }} emitidas
              </p>
            </div>

            <div
              class="grid gap-5 border-t border-primary-foreground/15 pt-5 sm:grid-cols-[1fr_auto] sm:items-end"
            >
              <div>
                <div class="mb-2 flex items-center justify-between text-xs">
                  <span class="text-primary-foreground/65">Tasa de recaudo</span>
                  <span class="metric-value font-semibold">
                    {{ financialSummary.collectionRate.toFixed(1) }}%
                  </span>
                </div>
                <div class="h-2 overflow-hidden rounded-full bg-primary-foreground/15">
                  <div
                    class="h-full rounded-full bg-primary-foreground transition-[width] duration-700"
                    :style="{ width: `${Math.min(financialSummary.collectionRate, 100)}%` }"
                  ></div>
                </div>
              </div>
              <div class="sm:text-right">
                <p class="text-xs text-primary-foreground/60">Total recaudado</p>
                <p class="metric-value mt-1 text-lg font-semibold">
                  ${{ formatCurrency(financialSummary.collected) }}
                </p>
              </div>
            </div>
          </div>
        </section>

        <aside class="surface client-section overflow-hidden xl:col-span-5">
          <div class="border-b p-5 sm:p-6">
            <div class="flex items-start justify-between gap-4">
              <div>
                <p class="font-mono text-[10px] font-semibold tracking-[0.14em] text-primary">
                  FICHA COMERCIAL
                </p>
                <h2 class="section-title mt-2">Datos de contacto</h2>
              </div>
              <div
                class="flex size-10 items-center justify-center rounded-xl bg-accent text-primary"
              >
                <ReceiptText class="size-5" :stroke-width="1.8" />
              </div>
            </div>
          </div>

          <dl class="divide-y px-5 sm:px-6">
            <div class="grid grid-cols-[auto_1fr] gap-3 py-4">
              <Mail class="mt-0.5 size-4 text-primary" />
              <div class="min-w-0">
                <dt class="text-xs text-muted-foreground">Correo electrónico</dt>
                <dd class="mt-1 truncate font-medium">{{ client.email || 'Sin registrar' }}</dd>
              </div>
            </div>
            <div class="grid grid-cols-[auto_1fr] gap-3 py-4">
              <Phone class="mt-0.5 size-4 text-primary" />
              <div>
                <dt class="text-xs text-muted-foreground">Teléfono</dt>
                <dd class="mt-1 font-medium">{{ client.phone || 'Sin registrar' }}</dd>
              </div>
            </div>
            <div class="grid grid-cols-[auto_1fr] gap-3 py-4">
              <MapPin class="mt-0.5 size-4 text-primary" />
              <div>
                <dt class="text-xs text-muted-foreground">Ubicación</dt>
                <dd class="mt-1 font-medium">{{ client.city || 'Ciudad sin registrar' }}</dd>
                <dd class="mt-1 text-sm text-muted-foreground">
                  {{ client.address || 'Dirección sin registrar' }}
                </dd>
              </div>
            </div>
            <div class="grid grid-cols-[auto_1fr] gap-3 py-4">
              <CalendarDays class="mt-0.5 size-4 text-primary" />
              <div>
                <dt class="text-xs text-muted-foreground">Relación registrada</dt>
                <dd class="mt-1 capitalize font-medium">{{ relationshipSince }}</dd>
              </div>
            </div>
          </dl>

          <div class="border-t bg-secondary/35 px-5 py-4 text-xs text-muted-foreground sm:px-6">
            <span v-if="lastActivityDate"
              >Último movimiento: {{ formatDate(lastActivityDate) }}</span
            >
            <span v-else>Aún no registra actividad financiera.</span>
          </div>
        </aside>
      </div>

      <section class="surface client-section overflow-hidden">
        <div class="grid sm:grid-cols-2 xl:grid-cols-4">
          <div
            class="flex min-h-32 flex-col justify-between border-b p-5 sm:border-r xl:border-b-0 sm:p-6"
          >
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Total emitido</p>
              <FileText class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <p class="metric-value mt-6 text-2xl font-semibold">
              ${{ formatCurrency(financialSummary.issued) }}
            </p>
          </div>
          <div
            class="flex min-h-32 flex-col justify-between border-b p-5 sm:border-r xl:border-b-0 sm:p-6"
          >
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Recaudado</p>
              <Banknote class="size-5 text-[hsl(var(--success))]" :stroke-width="1.8" />
            </div>
            <p class="metric-value mt-6 text-2xl font-semibold">
              ${{ formatCurrency(financialSummary.collected) }}
            </p>
          </div>
          <div
            class="flex min-h-32 flex-col justify-between border-b p-5 sm:border-r sm:p-6 xl:border-b-0"
          >
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Valor promedio</p>
              <ReceiptText class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <p class="metric-value mt-6 text-2xl font-semibold">
              ${{ formatCurrency(financialSummary.average) }}
            </p>
          </div>
          <div class="flex min-h-32 flex-col justify-between p-5 sm:p-6">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Documentos</p>
              <WalletCards class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <div class="mt-6 flex items-end justify-between gap-3">
              <p class="metric-value text-3xl font-semibold">{{ invoices.length }}</p>
              <p class="text-xs text-muted-foreground">histórico</p>
            </div>
          </div>
        </div>
      </section>

      <div class="grid gap-4 xl:grid-cols-12">
        <section
          class="surface client-section flex min-h-[390px] flex-col p-5 sm:p-6 xl:col-span-8"
        >
          <div>
            <p class="font-mono text-[10px] font-semibold tracking-[0.14em] text-primary">
              ÚLTIMOS 6 MESES
            </p>
            <h2 class="section-title mt-2">Actividad de la relación</h2>
            <p class="mt-1 text-sm text-muted-foreground">
              Compara lo emitido con los pagos recibidos en cada mes.
            </p>
          </div>
          <p class="sr-only">{{ chartSummary }}</p>
          <div v-if="invoices.length > 0" class="relative mt-6 min-h-64 flex-1">
            <Bar :data="barChartData" :options="barOptions" />
          </div>
          <div v-else class="flex flex-1 flex-col items-center justify-center py-10 text-center">
            <div class="empty-state-icon"><FileText class="size-5" /></div>
            <h3 class="font-semibold">Sin actividad para graficar</h3>
            <p class="mt-2 max-w-sm text-sm text-muted-foreground">
              La evolución aparecerá cuando emitas la primera cuenta para este cliente.
            </p>
          </div>
        </section>

        <section class="surface client-section p-5 sm:p-6 xl:col-span-4">
          <p class="font-mono text-[10px] font-semibold tracking-[0.14em] text-primary">
            COMPOSICIÓN
          </p>
          <h2 class="section-title mt-2">Estado de las cuentas</h2>
          <p class="mt-1 text-sm text-muted-foreground">Distribución del historial actual.</p>

          <div class="mt-7 space-y-7">
            <div v-for="row in statusRows" :key="row.status">
              <div class="flex items-start justify-between gap-4">
                <div class="flex min-w-0 gap-3">
                  <span class="mt-1.5 size-2 shrink-0 rounded-full" :class="row.dotClass"></span>
                  <div class="min-w-0">
                    <p class="font-medium">{{ row.label }}</p>
                    <p class="mt-0.5 truncate text-xs text-muted-foreground">
                      {{ row.description }}
                    </p>
                  </div>
                </div>
                <div class="shrink-0 text-right">
                  <p class="metric-value font-semibold">{{ row.count }}</p>
                  <p class="mt-0.5 text-[11px] text-muted-foreground">
                    ${{ formatCurrency(row.total) }}
                  </p>
                </div>
              </div>
              <div class="ml-5 mt-3 h-1.5 overflow-hidden rounded-full bg-secondary">
                <div
                  class="h-full rounded-full transition-[width] duration-700"
                  :class="row.barClass"
                  :style="{ width: `${row.percentage}%` }"
                ></div>
              </div>
            </div>
          </div>
        </section>
      </div>

      <div class="grid gap-4 xl:grid-cols-12">
        <section class="surface client-section overflow-hidden xl:col-span-8">
          <div class="flex items-start justify-between gap-4 border-b p-5 sm:p-6">
            <div>
              <h2 class="section-title">Historial de cuentas</h2>
              <p class="mt-1 text-sm text-muted-foreground">
                Todos los documentos emitidos al cliente.
              </p>
            </div>
            <span
              class="font-mono text-[10px] font-semibold tracking-[0.12em] text-muted-foreground"
            >
              {{ invoices.length }} REGISTROS
            </span>
          </div>

          <div v-if="invoices.length > 0" class="hidden overflow-x-auto md:block">
            <table class="data-table min-w-[760px]">
              <thead>
                <tr>
                  <th>Cuenta</th>
                  <th>Estado</th>
                  <th class="text-right">Total</th>
                  <th class="text-right">Recaudado</th>
                  <th class="text-right">Saldo</th>
                  <th class="w-24 text-right">Acciones</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="invoice in invoices" :key="invoice.id">
                  <td>
                    <p class="font-mono text-xs font-semibold text-primary">
                      #{{ invoice.number }}
                    </p>
                    <p class="mt-1 text-xs text-muted-foreground">{{ formatDate(invoice.date) }}</p>
                  </td>
                  <td><StatusBadge :status="invoice.status" /></td>
                  <td class="metric-value text-right font-semibold">
                    ${{ formatCurrency(invoice.total) }}
                  </td>
                  <td class="metric-value text-right text-muted-foreground">
                    ${{ formatCurrency(invoice.paid_amount) }}
                  </td>
                  <td class="metric-value text-right font-semibold">
                    ${{ formatCurrency(invoice.balance) }}
                  </td>
                  <td>
                    <div class="flex justify-end gap-1.5">
                      <button
                        type="button"
                        class="icon-button"
                        :aria-label="`Editar cuenta ${invoice.number}`"
                        title="Editar"
                        @click="router.push(`/invoices/edit/${invoice.id}`)"
                      >
                        <Pencil />
                      </button>
                      <button
                        type="button"
                        class="icon-button"
                        :aria-label="`Imprimir cuenta ${invoice.number}`"
                        title="Imprimir"
                        @click="router.push(`/print/${invoice.id}`)"
                      >
                        <Printer />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div v-if="invoices.length > 0" class="divide-y md:hidden">
            <article v-for="invoice in invoices" :key="invoice.id" class="p-5">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <p class="font-mono text-xs font-semibold text-primary">#{{ invoice.number }}</p>
                  <p class="mt-1 text-xs text-muted-foreground">{{ formatDate(invoice.date) }}</p>
                </div>
                <StatusBadge :status="invoice.status" />
              </div>
              <div class="mt-4 grid grid-cols-2 gap-3 rounded-xl bg-secondary/55 p-4 text-sm">
                <div>
                  <p class="text-xs text-muted-foreground">Total</p>
                  <p class="metric-value mt-1 font-semibold">
                    ${{ formatCurrency(invoice.total) }}
                  </p>
                </div>
                <div class="text-right">
                  <p class="text-xs text-muted-foreground">Saldo</p>
                  <p class="metric-value mt-1 font-semibold">
                    ${{ formatCurrency(invoice.balance) }}
                  </p>
                </div>
              </div>
              <div class="mt-3 grid grid-cols-2 gap-2">
                <Button variant="outline" @click="router.push(`/invoices/edit/${invoice.id}`)">
                  <Pencil /> Editar
                </Button>
                <Button variant="outline" @click="router.push(`/print/${invoice.id}`)">
                  <Printer /> Imprimir
                </Button>
              </div>
            </article>
          </div>

          <div v-if="invoices.length === 0" class="empty-state">
            <div class="empty-state-icon"><FileText class="size-5" /></div>
            <h3 class="section-title">Aún no hay cuentas</h3>
            <p class="mt-2 max-w-md text-sm text-muted-foreground">
              Emite el primer documento para comenzar el historial financiero.
            </p>
            <Button class="mt-5" @click="router.push(`/invoices/new?clientId=${client.id}`)">
              <Plus /> Nueva cuenta
            </Button>
          </div>
        </section>

        <aside class="surface client-section overflow-hidden xl:col-span-4">
          <div class="border-b p-5 sm:p-6">
            <p class="font-mono text-[10px] font-semibold tracking-[0.14em] text-primary">
              RECAUDO
            </p>
            <h2 class="section-title mt-2">Últimos abonos</h2>
            <p class="mt-1 text-sm text-muted-foreground">Movimientos recibidos recientemente.</p>
          </div>

          <div v-if="recentPayments.length > 0" class="divide-y px-5 sm:px-6">
            <RouterLink
              v-for="payment in recentPayments"
              :key="payment.id"
              :to="`/invoices/edit/${payment.invoice_id}`"
              class="group grid grid-cols-[auto_1fr_auto] items-center gap-3 py-4"
            >
              <div
                class="flex size-9 items-center justify-center rounded-lg bg-accent text-primary"
              >
                <Banknote class="size-4" :stroke-width="1.8" />
              </div>
              <div class="min-w-0">
                <p class="text-sm font-medium">Cuenta #{{ payment.invoice_number }}</p>
                <p class="mt-0.5 truncate text-xs text-muted-foreground">
                  {{ formatDate(payment.date) }} · {{ payment.notes || 'Abono registrado' }}
                </p>
              </div>
              <div class="flex items-center gap-2">
                <p class="metric-value text-sm font-semibold text-[hsl(var(--success))]">
                  +${{ formatCurrency(payment.amount) }}
                </p>
                <ArrowRight
                  class="size-3.5 text-muted-foreground transition-transform group-hover:translate-x-0.5"
                />
              </div>
            </RouterLink>
          </div>

          <div v-else class="empty-state min-h-64">
            <div class="empty-state-icon"><Banknote class="size-5" /></div>
            <h3 class="font-semibold">Sin abonos registrados</h3>
            <p class="mt-2 max-w-xs text-sm text-muted-foreground">
              Los pagos aplicados a sus cuentas aparecerán aquí.
            </p>
          </div>
        </aside>
      </div>
    </template>
  </div>
</template>

<style scoped>
.client-hero,
.client-section {
  animation: client-detail-enter 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
}

.client-section:nth-of-type(2) {
  animation-delay: 55ms;
}

.client-orbit {
  position: absolute;
  width: 25rem;
  height: 25rem;
  right: -10rem;
  bottom: -14rem;
  border: 1px solid hsl(var(--primary-foreground) / 0.14);
  border-radius: 50%;
  box-shadow:
    0 0 0 3.5rem hsl(var(--primary-foreground) / 0.035),
    0 0 0 8rem hsl(var(--primary-foreground) / 0.025);
}

@keyframes client-detail-enter {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
