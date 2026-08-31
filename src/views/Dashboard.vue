<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import {
  ArrowDownRight,
  ArrowRight,
  ArrowUpRight,
  Banknote,
  FileText,
  Plus,
  ReceiptText,
  RotateCcw,
  TrendingUp,
  Users,
  WalletCards,
} from '@lucide/vue'
import { Bar } from 'vue-chartjs'
import {
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Legend,
  LinearScale,
  Title,
  Tooltip,
} from 'chart.js'
import type { TooltipItem } from 'chart.js'
import PageHeader from '@/components/PageHeader.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/toast/use-toast'
import { useTheme } from '@/composables/useTheme'
import { formatCurrency, formatDate } from '@/lib/format'
import type { Invoice } from '@/types'

ChartJS.register(Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale)

interface InvoiceSummaryRow {
  count: number
  total: number
  average: number
}

interface CountRow {
  count: number
}

interface AmountRow {
  total: number
}

interface PeriodRow extends AmountRow {
  count: number
}

interface StatusRow extends AmountRow {
  status: Invoice['status']
  count: number
}

interface MonthlyRow extends AmountRow {
  month: string
}

interface TopClientRow {
  id: number
  name: string
  invoice_count: number
  total: number
}

const router = useRouter()
const { toast } = useToast()
const { resolvedTheme } = useTheme()

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
const isLoading = ref(true)
const loadError = ref(false)
const stats = ref({
  totalInvoices: 0,
  totalClients: 0,
  totalAmount: 0,
  averageAmount: 0,
  collectedAmount: 0,
  currentMonthAmount: 0,
  currentMonthCount: 0,
  previousMonthAmount: 0,
})
const statusData = ref<StatusRow[]>([])
const monthlyInvoices = ref<MonthlyRow[]>([])
const monthlyPayments = ref<MonthlyRow[]>([])
const recentInvoices = ref<Invoice[]>([])
const topClients = ref<TopClientRow[]>([])

const currentDateLabel = new Intl.DateTimeFormat('es-CO', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
}).format(new Date())

const currentMonthLabel = new Intl.DateTimeFormat('es-CO', { month: 'long' }).format(new Date())

const outstandingAmount = computed(() =>
  Math.max(stats.value.totalAmount - stats.value.collectedAmount, 0),
)
const collectionRate = computed(() =>
  stats.value.totalAmount > 0
    ? Math.min((stats.value.collectedAmount / stats.value.totalAmount) * 100, 100)
    : 0,
)
const monthlyChange = computed<number | null>(() => {
  if (stats.value.previousMonthAmount === 0) return null
  return (
    ((stats.value.currentMonthAmount - stats.value.previousMonthAmount) /
      stats.value.previousMonthAmount) *
    100
  )
})
const isMonthlyGrowth = computed(() => (monthlyChange.value ?? 0) >= 0)

const statusMeta: Record<
  Invoice['status'],
  { label: string; description: string; dotClass: string; barClass: string }
> = {
  paid: {
    label: 'Pagadas',
    description: 'Cuentas recaudadas por completo',
    dotClass: 'bg-[hsl(var(--success))]',
    barClass: 'bg-[hsl(var(--success))]',
  },
  partially_paid: {
    label: 'Con abonos',
    description: 'Cuentas con pagos parciales',
    dotClass: 'bg-[hsl(var(--info))]',
    barClass: 'bg-[hsl(var(--info))]',
  },
  draft: {
    label: 'Pendientes',
    description: 'Cuentas sin recaudo completo',
    dotClass: 'bg-[hsl(var(--warning))]',
    barClass: 'bg-[hsl(var(--warning))]',
  },
}

const statusRows = computed(() => {
  const total = stats.value.totalInvoices
  return (['paid', 'partially_paid', 'draft'] as const).map((status) => {
    const row = statusData.value.find((item) => item.status === status)
    return {
      status,
      count: row?.count ?? 0,
      total: row?.total ?? 0,
      percentage: total > 0 ? ((row?.count ?? 0) / total) * 100 : 0,
      ...statusMeta[status],
    }
  })
})

const lastSixMonths = computed(() => {
  const now = new Date()
  return Array.from({ length: 6 }, (_, index) => {
    const date = new Date(now.getFullYear(), now.getMonth() - 5 + index, 1)
    const month = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`
    return {
      month,
      label: new Intl.DateTimeFormat('es-CO', { month: 'short' })
        .format(date)
        .replace('.', ''),
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

const largestClientTotal = computed(() => topClients.value[0]?.total ?? 0)

const loadDashboard = async () => {
  isLoading.value = true
  loadError.value = false

  try {
    const [
      invoiceSummary,
      clientSummary,
      paymentSummary,
      currentMonth,
      previousMonth,
      statuses,
      invoicesByMonth,
      paymentsByMonth,
      recent,
      leadingClients,
    ] = await Promise.all([
      window.electronAPI.dbQuery<InvoiceSummaryRow>(`
        SELECT COUNT(*) as count,
          COALESCE(SUM(total), 0) as total,
          COALESCE(AVG(total), 0) as average
        FROM invoices
      `),
      window.electronAPI.dbQuery<CountRow>('SELECT COUNT(*) as count FROM clients'),
      window.electronAPI.dbQuery<AmountRow>(
        'SELECT COALESCE(SUM(amount), 0) as total FROM invoice_payments',
      ),
      window.electronAPI.dbQuery<PeriodRow>(`
        SELECT COUNT(*) as count, COALESCE(SUM(total), 0) as total
        FROM invoices
        WHERE date >= date('now', 'localtime', 'start of month')
      `),
      window.electronAPI.dbQuery<PeriodRow>(`
        SELECT COUNT(*) as count, COALESCE(SUM(total), 0) as total
        FROM invoices
        WHERE date >= date('now', 'localtime', 'start of month', '-1 month')
          AND date < date('now', 'localtime', 'start of month')
      `),
      window.electronAPI.dbQuery<StatusRow>(`
        SELECT status, COUNT(*) as count, COALESCE(SUM(total), 0) as total
        FROM invoices
        GROUP BY status
      `),
      window.electronAPI.dbQuery<MonthlyRow>(`
        SELECT strftime('%Y-%m', date) as month, COALESCE(SUM(total), 0) as total
        FROM invoices
        WHERE date >= date('now', 'localtime', 'start of month', '-5 months')
        GROUP BY month
        ORDER BY month
      `),
      window.electronAPI.dbQuery<MonthlyRow>(`
        SELECT strftime('%Y-%m', date) as month, COALESCE(SUM(amount), 0) as total
        FROM invoice_payments
        WHERE date >= date('now', 'localtime', 'start of month', '-5 months')
        GROUP BY month
        ORDER BY month
      `),
      window.electronAPI.dbQuery<Invoice>(`
        SELECT i.*, c.name as client_name,
          COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0) as paid_amount,
          MAX(i.total - COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0), 0) as balance
        FROM invoices i
        JOIN clients c ON c.id = i.client_id
        ORDER BY i.date DESC, i.number DESC
        LIMIT 5
      `),
      window.electronAPI.dbQuery<TopClientRow>(`
        SELECT c.id, c.name, COUNT(i.id) as invoice_count, COALESCE(SUM(i.total), 0) as total
        FROM clients c
        JOIN invoices i ON i.client_id = c.id
        GROUP BY c.id, c.name
        ORDER BY total DESC
        LIMIT 5
      `),
    ])

    const invoiceTotals = invoiceSummary[0]
    stats.value = {
      totalInvoices: invoiceTotals?.count ?? 0,
      totalClients: clientSummary[0]?.count ?? 0,
      totalAmount: invoiceTotals?.total ?? 0,
      averageAmount: invoiceTotals?.average ?? 0,
      collectedAmount: paymentSummary[0]?.total ?? 0,
      currentMonthAmount: currentMonth[0]?.total ?? 0,
      currentMonthCount: currentMonth[0]?.count ?? 0,
      previousMonthAmount: previousMonth[0]?.total ?? 0,
    }
    statusData.value = statuses
    monthlyInvoices.value = invoicesByMonth
    monthlyPayments.value = paymentsByMonth
    recentInvoices.value = recent
    topClients.value = leadingClients
  } catch {
    loadError.value = true
    toast({
      title: 'No se pudo cargar el panel',
      description: 'La base de datos no respondió correctamente.',
      variant: 'destructive',
    })
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  chartColors.value = readChartColors()
  loadDashboard()
})

watch(resolvedTheme, async () => {
  await nextTick()
  chartColors.value = readChartColors()
})
</script>

<template>
  <div class="app-page">
    <PageHeader
      title="Panel financiero"
      :description="`Una lectura clara de tu operación al ${currentDateLabel}.`"
    >
      <template #leading>
        <div class="inline-flex items-center gap-2 text-xs font-medium text-muted-foreground">
          <span class="relative flex size-2">
            <span class="absolute inline-flex size-full animate-ping rounded-full bg-primary/50"></span>
            <span class="relative inline-flex size-2 rounded-full bg-primary"></span>
          </span>
          Datos actualizados
        </div>
      </template>
      <template #actions>
        <Button class="hidden md:inline-flex" @click="router.push('/invoices/new')">
          <Plus class="size-4" />
          Nueva cuenta
        </Button>
      </template>
    </PageHeader>

    <section
      v-if="loadError"
      class="surface flex flex-col gap-4 p-6 sm:flex-row sm:items-center sm:justify-between"
    >
      <div>
        <h2 class="section-title">No pudimos cargar el panel</h2>
        <p class="mt-1 text-sm text-muted-foreground">
          Comprueba la base de datos e inténtalo de nuevo.
        </p>
      </div>
      <Button variant="outline" @click="loadDashboard"><RotateCcw /> Reintentar</Button>
    </section>

    <div v-else-if="isLoading" class="grid animate-pulse gap-4 xl:grid-cols-12" aria-busy="true">
      <div class="h-[350px] rounded-[1.5rem] bg-secondary xl:col-span-7"></div>
      <div class="grid gap-4 sm:grid-cols-2 xl:col-span-5">
        <div class="h-44 rounded-[1.125rem] bg-secondary sm:col-span-2"></div>
        <div class="h-40 rounded-[1.125rem] bg-secondary"></div>
        <div class="h-40 rounded-[1.125rem] bg-secondary"></div>
      </div>
      <div class="h-[390px] rounded-[1.125rem] bg-secondary xl:col-span-8"></div>
      <div class="h-[390px] rounded-[1.125rem] bg-secondary xl:col-span-4"></div>
    </div>

    <section v-else-if="stats.totalInvoices === 0" class="surface empty-state min-h-[420px]">
      <div class="empty-state-icon"><FileText class="size-5" /></div>
      <p class="mb-3 font-mono text-xs font-semibold tracking-[0.14em] text-primary">
        PANEL LISTO
      </p>
      <h2 class="text-2xl font-semibold tracking-[-0.04em]">Tu actividad empieza aquí</h2>
      <p class="mt-3 max-w-md text-sm leading-6 text-muted-foreground">
        Crea la primera cuenta de cobro. El panel calculará automáticamente recaudo, cartera,
        clientes principales y evolución mensual.
      </p>
      <Button class="mt-6" @click="router.push('/invoices/new')"><Plus /> Nueva cuenta</Button>
    </section>

    <template v-else>
      <div class="grid gap-4 xl:grid-cols-12">
        <section
          class="financial-hero dashboard-section relative isolate overflow-hidden rounded-[1.5rem] bg-primary p-6 text-primary-foreground sm:p-8 xl:col-span-7"
        >
          <div class="relative z-10 flex min-h-[286px] flex-col justify-between gap-10">
            <div class="flex items-start justify-between gap-4">
              <div>
                <div class="flex items-center gap-2 text-sm font-medium text-primary-foreground/70">
                  <WalletCards class="size-4" :stroke-width="1.8" />
                  Cartera por cobrar
                </div>
                <p class="mt-3 max-w-md text-sm leading-6 text-primary-foreground/60">
                  Saldo acumulado de las cuentas emitidas que todavía no ha ingresado.
                </p>
              </div>
              <span
                class="shrink-0 rounded-lg border border-primary-foreground/15 bg-primary-foreground/10 px-2.5 py-1 font-mono text-[10px] font-semibold tracking-[0.12em]"
              >
                COP
              </span>
            </div>

            <div>
              <p class="metric-value text-[clamp(2.3rem,5vw,4.25rem)] font-semibold leading-none">
                ${{ formatCurrency(outstandingAmount) }}
              </p>
              <p class="mt-3 text-sm text-primary-foreground/60">
                de ${{ formatCurrency(stats.totalAmount) }} emitidos históricamente
              </p>
            </div>

            <div class="grid gap-5 border-t border-primary-foreground/15 pt-5 sm:grid-cols-[1fr_auto] sm:items-end">
              <div>
                <div class="mb-2 flex items-center justify-between text-xs">
                  <span class="text-primary-foreground/65">Tasa de recaudo</span>
                  <span class="metric-value font-semibold">{{ collectionRate.toFixed(1) }}%</span>
                </div>
                <div class="h-2 overflow-hidden rounded-full bg-primary-foreground/15">
                  <div
                    class="h-full rounded-full bg-primary-foreground transition-[width] duration-700"
                    :style="{ width: `${collectionRate}%` }"
                  ></div>
                </div>
              </div>
              <div class="sm:text-right">
                <p class="text-xs text-primary-foreground/60">Recaudado</p>
                <p class="metric-value mt-1 text-lg font-semibold">
                  ${{ formatCurrency(stats.collectedAmount) }}
                </p>
              </div>
            </div>
          </div>
        </section>

        <aside class="grid gap-4 sm:grid-cols-2 xl:col-span-5">
          <section class="surface dashboard-section relative overflow-hidden p-6 sm:col-span-2">
            <div class="flex items-start justify-between gap-4">
              <div>
                <p class="text-sm font-medium text-muted-foreground">Emitido en {{ currentMonthLabel }}</p>
                <p class="metric-value mt-5 text-3xl font-semibold sm:text-4xl">
                  ${{ formatCurrency(stats.currentMonthAmount) }}
                </p>
              </div>
              <div class="flex size-11 items-center justify-center rounded-xl bg-accent text-primary">
                <TrendingUp class="size-5" :stroke-width="1.8" />
              </div>
            </div>
            <div class="mt-6 flex flex-wrap items-center gap-x-3 gap-y-2 border-t pt-4 text-sm">
              <span
                v-if="monthlyChange !== null"
                class="inline-flex items-center gap-1 font-semibold"
                :class="isMonthlyGrowth ? 'text-[hsl(var(--success))]' : 'text-destructive'"
              >
                <ArrowUpRight v-if="isMonthlyGrowth" class="size-4" />
                <ArrowDownRight v-else class="size-4" />
                {{ Math.abs(monthlyChange).toFixed(1) }}%
              </span>
              <span v-else class="font-medium text-muted-foreground">Sin mes comparable</span>
              <span class="text-muted-foreground">
                {{ stats.currentMonthCount }}
                {{ stats.currentMonthCount === 1 ? 'cuenta emitida' : 'cuentas emitidas' }}
              </span>
            </div>
          </section>

          <section class="surface dashboard-section flex min-h-40 flex-col justify-between p-5 sm:p-6">
            <div class="flex items-center justify-between">
              <p class="text-sm font-medium text-muted-foreground">Valor promedio</p>
              <ReceiptText class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <div>
              <p class="metric-value text-2xl font-semibold sm:text-3xl">
                ${{ formatCurrency(stats.averageAmount) }}
              </p>
              <p class="mt-2 text-xs text-muted-foreground">por cuenta emitida</p>
            </div>
          </section>

          <section class="surface dashboard-section flex min-h-40 flex-col justify-between p-5 sm:p-6">
            <div class="flex items-center justify-between">
              <p class="text-sm font-medium text-muted-foreground">Base comercial</p>
              <Users class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <div class="flex items-end justify-between gap-4">
              <div>
                <p class="metric-value text-3xl font-semibold">{{ stats.totalClients }}</p>
                <p class="mt-2 text-xs text-muted-foreground">clientes registrados</p>
              </div>
              <p class="font-mono text-xs text-muted-foreground">
                {{ stats.totalInvoices }} docs.
              </p>
            </div>
          </section>
        </aside>
      </div>

      <div class="grid gap-4 xl:grid-cols-12">
        <section class="surface dashboard-section flex min-h-[390px] flex-col p-5 sm:p-6 xl:col-span-8">
          <div class="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <p class="font-mono text-[10px] font-semibold tracking-[0.14em] text-primary">
                ÚLTIMOS 6 MESES
              </p>
              <h2 class="section-title mt-2">Emitido frente a recaudado</h2>
              <p class="mt-1 text-sm text-muted-foreground">
                Compara el valor de las cuentas con el dinero recibido cada mes.
              </p>
            </div>
            <RouterLink
              to="/invoices"
              class="group inline-flex items-center gap-1.5 text-sm font-medium text-primary transition-colors hover:text-primary/75"
            >
              Ver cuentas
              <ArrowRight class="size-4 transition-transform group-hover:translate-x-0.5" />
            </RouterLink>
          </div>
          <p class="sr-only">{{ chartSummary }}</p>
          <div class="relative mt-6 min-h-0 flex-1">
            <Bar :data="barChartData" :options="barOptions" />
          </div>
        </section>

        <section class="surface dashboard-section p-5 sm:p-6 xl:col-span-4">
          <div>
            <p class="font-mono text-[10px] font-semibold tracking-[0.14em] text-primary">
              PORTAFOLIO
            </p>
            <h2 class="section-title mt-2">Estado de las cuentas</h2>
            <p class="mt-1 text-sm text-muted-foreground">Distribución de los documentos actuales.</p>
          </div>

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
                    {{ row.percentage.toFixed(0) }}%
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
        <section class="surface dashboard-section overflow-hidden xl:col-span-7">
          <div class="flex items-start justify-between gap-4 border-b p-5 sm:p-6">
            <div>
              <h2 class="section-title">Cuentas recientes</h2>
              <p class="mt-1 text-sm text-muted-foreground">Últimos movimientos emitidos.</p>
            </div>
            <Banknote class="size-5 text-primary" :stroke-width="1.8" />
          </div>
          <div class="divide-y">
            <RouterLink
              v-for="invoice in recentInvoices"
              :key="invoice.id"
              :to="`/invoices/edit/${invoice.id}`"
              class="group grid gap-3 px-5 py-4 transition-colors hover:bg-accent/35 sm:grid-cols-[minmax(0,1fr)_auto_auto] sm:items-center sm:px-6"
            >
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span class="font-mono text-xs font-semibold text-primary">#{{ invoice.number }}</span>
                  <span class="text-xs text-muted-foreground">{{ formatDate(invoice.date) }}</span>
                </div>
                <p class="mt-1 truncate font-medium">{{ invoice.client_name }}</p>
              </div>
              <StatusBadge :status="invoice.status" class="w-fit" />
              <div class="flex items-center justify-between gap-4 sm:min-w-32 sm:text-right">
                <div>
                  <p class="metric-value font-semibold">${{ formatCurrency(invoice.total) }}</p>
                  <p v-if="(invoice.balance ?? 0) > 0" class="mt-0.5 text-[11px] text-muted-foreground">
                    Saldo ${{ formatCurrency(invoice.balance) }}
                  </p>
                  <p v-else class="mt-0.5 text-[11px] text-[hsl(var(--success))]">Sin saldo</p>
                </div>
                <ArrowRight
                  class="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-primary"
                />
              </div>
            </RouterLink>
          </div>
        </section>

        <section class="surface dashboard-section p-5 sm:p-6 xl:col-span-5">
          <div class="flex items-start justify-between gap-4">
            <div>
              <h2 class="section-title">Clientes principales</h2>
              <p class="mt-1 text-sm text-muted-foreground">Participación por valor emitido.</p>
            </div>
            <Users class="size-5 text-primary" :stroke-width="1.8" />
          </div>

          <div class="mt-7 space-y-5">
            <div v-for="(client, index) in topClients" :key="client.id">
              <div class="flex items-end justify-between gap-4">
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="font-mono text-[10px] text-muted-foreground">
                      {{ String(index + 1).padStart(2, '0') }}
                    </span>
                    <p class="truncate font-medium">{{ client.name }}</p>
                  </div>
                  <p class="ml-7 mt-1 text-xs text-muted-foreground">
                    {{ client.invoice_count }}
                    {{ client.invoice_count === 1 ? 'cuenta' : 'cuentas' }}
                  </p>
                </div>
                <p class="metric-value shrink-0 text-sm font-semibold">
                  ${{ formatCurrency(client.total) }}
                </p>
              </div>
              <div class="ml-7 mt-2 h-1 overflow-hidden rounded-full bg-secondary">
                <div
                  class="h-full rounded-full bg-primary/75"
                  :style="{
                    width: `${largestClientTotal > 0 ? (client.total / largestClientTotal) * 100 : 0}%`,
                  }"
                ></div>
              </div>
            </div>
          </div>
        </section>
      </div>
    </template>
  </div>
</template>

<style scoped>
.financial-hero::before {
  position: absolute;
  width: 22rem;
  height: 22rem;
  border: 1px solid hsl(var(--primary-foreground) / 0.11);
  border-radius: 999px;
  content: '';
  right: -7rem;
  top: -10rem;
  box-shadow:
    0 0 0 3rem hsl(var(--primary-foreground) / 0.035),
    0 0 0 7rem hsl(var(--primary-foreground) / 0.025);
}

.dashboard-section {
  animation: dashboard-rise 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
}

.dashboard-section:nth-child(2) {
  animation-delay: 45ms;
}

.dashboard-section:nth-child(3) {
  animation-delay: 90ms;
}

@keyframes dashboard-rise {
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
