<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { FileText, Users, DollarSign, Activity } from 'lucide-vue-next'
import RetroSpinner from '@/components/ui/animations/RetroSpinner.vue'
import DataWaves from '@/components/ui/animations/DataWaves.vue'
import ScanningLine from '@/components/ui/animations/ScanningLine.vue'
import BlinkingCursor from '@/components/ui/animations/BlinkingCursor.vue'
import { Bar, Doughnut } from 'vue-chartjs'
import {
  Chart as ChartJS,
  Title,
  Tooltip,
  Legend,
  BarElement,
  CategoryScale,
  LinearScale,
  ArcElement
} from 'chart.js'

ChartJS.register(Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale, ArcElement)

const stats = ref({
  totalInvoices: 0,
  totalClients: 0,
  totalAmount: 0,
})

const isLoading = ref(true)

const statusData = ref<{ status: string; count: number }[]>([])
const monthlyData = ref<{ month: string; total: number }[]>([])

const doughnutChartData = computed(() => {
  const map: Record<string, number> = {
    paid: 0,
    partially_paid: 0,
    draft: 0
  }
  statusData.value.forEach(item => {
    map[item.status] = item.count
  })

  return {
    labels: ['Pagadas', 'Abonadas', 'Pendientes'],
    datasets: [
      {
        backgroundColor: ['#4ade80', '#fb923c', '#facc15'],
        borderColor: ['#000000', '#000000', '#000000'],
        borderWidth: 3,
        data: [map.paid, map.partially_paid, map.draft]
      }
    ]
  }
})

const doughnutOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      position: 'bottom' as const,
      labels: {
        font: { family: 'monospace', weight: 'bold' as const },
        color: '#000'
      }
    }
  }
}

const barChartData = computed(() => {
  // Sort ascending for chart
  const sorted = [...monthlyData.value].reverse()
  return {
    labels: sorted.map(d => {
      const [year, month] = d.month.split('-')
      const date = new Date(parseInt(year), parseInt(month) - 1)
      return date.toLocaleDateString('es-ES', { month: 'short', year: '2-digit' }).toUpperCase()
    }),
    datasets: [
      {
        label: 'Ingresos Facturados ($)',
        backgroundColor: '#000000',
        borderColor: '#000000',
        borderWidth: 2,
        data: sorted.map(d => d.total)
      }
    ]
  }
})

const barOptions = {
  responsive: true,
  maintainAspectRatio: false,
  scales: {
    y: {
      beginAtZero: true,
      grid: { color: '#e5e7eb' },
      ticks: { font: { family: 'monospace', weight: 'bold' as const }, color: '#000' }
    },
    x: {
      grid: { display: false },
      ticks: { font: { family: 'monospace', weight: 'bold' as const }, color: '#000' }
    }
  },
  plugins: {
    legend: {
      position: 'top' as const,
      labels: { font: { family: 'monospace', weight: 'bold' as const }, color: '#000' }
    }
  }
}

onMounted(async () => {
  try {
    const invoices = await window.electronAPI.dbQuery<{ count: number; total: number | null }>(
      'SELECT COUNT(*) as count, SUM(total) as total FROM invoices',
    )
    const clients = await window.electronAPI.dbQuery<{ count: number }>(
      'SELECT COUNT(*) as count FROM clients',
    )

    const statuses = await window.electronAPI.dbQuery<{ status: string; count: number }>(
      'SELECT status, COUNT(*) as count FROM invoices GROUP BY status'
    )
    statusData.value = statuses

    const monthly = await window.electronAPI.dbQuery<{ month: string; total: number }>(
      "SELECT strftime('%Y-%m', date) as month, SUM(total) as total FROM invoices GROUP BY month ORDER BY month DESC LIMIT 6"
    )
    monthlyData.value = monthly

    stats.value = {
      totalInvoices: invoices[0].count || 0,
      totalClients: clients[0].count || 0,
      totalAmount: invoices[0].total || 0,
    }
  } catch (e) {
    console.error(e)
  } finally {
    isLoading.value = false
  }
})
</script>

<template>
  <div class="space-y-12">
    <!-- Header -->
    <div class="border-b-[4px] border-foreground pb-6 mb-8 flex flex-col md:flex-row md:items-end justify-between gap-6 relative overflow-hidden group">
      <ScanningLine />
      <div>
        <h2 class="text-5xl md:text-7xl font-black tracking-tighter uppercase leading-none mb-3">Dashboard</h2>
        <div class="flex items-center gap-3">
          <div class="h-3 w-3 rounded-full bg-accent animate-pulse border border-foreground"></div>
          <p class="font-mono text-xs font-bold tracking-widest text-muted-foreground uppercase flex items-center">
            Resumen Operativo / {{ new Date().toLocaleDateString('es-ES') }}
            <BlinkingCursor />
          </p>
        </div>
      </div>
      
      <!-- Decorative spinning element -->
      <div class="hidden md:flex w-20 h-20 bg-accent border-[3px] border-foreground shadow-[4px_4px_0_0_hsl(var(--foreground))] items-center justify-center relative overflow-hidden">
        <div class="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI0IiBoZWlnaHQ9IjQiPgo8cmVjdCB3aWR0aD0iNCIgaGVpZ2h0PSI0IiBmaWxsPSJ0cmFuc3BhcmVudCIvPgo8Y2lyY2xlIGN4PSIyIiBjeT0iMiIgcj0iMSIgZmlsbD0icmdiYSgyNTUsMjU1LDI1NSwwLjEpIi8+Cjwvc3ZnPg==')] opacity-30"></div>
        <RetroSpinner size="40px" />
      </div>
    </div>

    <!-- Stats Grid -->
    <div class="grid gap-8 md:grid-cols-3">
      
      <!-- Stat Card 1 -->
      <div class="border-[3px] border-foreground p-6 bg-card relative group transition-all hover:-translate-y-2 hover:-translate-x-2 hover:shadow-[12px_12px_0_0_hsl(var(--accent))] shadow-[6px_6px_0_0_hsl(var(--foreground))] overflow-hidden">
        <DataWaves />
        <div class="absolute -right-6 -top-6 w-24 h-24 bg-secondary rounded-full -z-10 group-hover:bg-accent/20 transition-colors"></div>
        <div class="absolute top-0 right-0 px-2 py-1 bg-foreground text-background font-mono text-[0.65rem] font-bold border-b-[3px] border-l-[3px] border-foreground">STAT_01</div>
        
        <div class="flex flex-row items-center justify-between mb-8 mt-2">
          <h3 class="text-xl font-bold uppercase tracking-wider">Total Cuentas</h3>
          <FileText class="h-8 w-8 text-foreground" />
        </div>
        <div>
          <div class="text-6xl font-black tracking-tighter" v-if="!isLoading">{{ stats.totalInvoices }}</div>
          <div class="flex items-center justify-center py-2" v-else>
            <RetroSpinner size="48px" />
          </div>
        </div>
      </div>

      <!-- Stat Card 2 -->
      <div class="border-[3px] border-foreground p-6 bg-card relative group transition-all hover:-translate-y-2 hover:-translate-x-2 hover:shadow-[12px_12px_0_0_hsl(var(--accent))] shadow-[6px_6px_0_0_hsl(var(--foreground))] overflow-hidden">
        <DataWaves />
        <div class="absolute -right-6 -top-6 w-24 h-24 bg-secondary rounded-full -z-10 group-hover:bg-accent/20 transition-colors"></div>
        <div class="absolute top-0 right-0 px-2 py-1 bg-foreground text-background font-mono text-[0.65rem] font-bold border-b-[3px] border-l-[3px] border-foreground">STAT_02</div>
        
        <div class="flex flex-row items-center justify-between mb-8 mt-2">
          <h3 class="text-xl font-bold uppercase tracking-wider">Clientes</h3>
          <Users class="h-8 w-8 text-foreground" />
        </div>
        <div>
          <div class="text-6xl font-black tracking-tighter" v-if="!isLoading">{{ stats.totalClients }}</div>
          <div class="flex items-center justify-center py-2" v-else>
            <RetroSpinner size="48px" />
          </div>
        </div>
      </div>

      <!-- Stat Card 3 -->
      <div class="border-[3px] border-foreground p-6 bg-accent text-white relative group transition-all hover:-translate-y-2 hover:-translate-x-2 hover:shadow-[12px_12px_0_0_hsl(var(--foreground))] shadow-[6px_6px_0_0_hsl(var(--foreground))] overflow-hidden">
        <DataWaves />
        <div class="absolute -right-6 -top-6 w-24 h-24 bg-white/20 rounded-full -z-10"></div>
        <div class="absolute top-0 right-0 px-2 py-1 bg-white text-accent font-mono text-[0.65rem] font-bold border-b-[3px] border-l-[3px] border-foreground">STAT_03</div>
        
        <div class="flex flex-row items-center justify-between mb-8 mt-2">
          <h3 class="text-xl font-bold uppercase tracking-wider text-white">Monto Total</h3>
          <DollarSign class="h-8 w-8 text-white" />
        </div>
        <div>
          <div class="text-5xl md:text-6xl font-black tracking-tighter" v-if="!isLoading">
            <span class="text-3xl opacity-80 mr-1">$</span>{{ stats.totalAmount.toLocaleString() }}
          </div>
          <div class="flex items-center justify-center py-2" v-else>
            <RetroSpinner size="48px" />
          </div>
        </div>
      </div>

    </div>

    <!-- Charts Grid -->
    <div class="grid gap-8 md:grid-cols-3 mt-12" v-if="!isLoading && (statusData.length > 0 || monthlyData.length > 0)">
      
      <!-- Status Doughnut Chart -->
      <div class="md:col-span-1 border-[3px] border-foreground p-6 bg-card relative shadow-[6px_6px_0_0_hsl(var(--foreground))] h-[400px] flex flex-col">
        <div class="absolute top-0 right-0 px-2 py-1 bg-foreground text-background font-mono text-[0.65rem] font-bold border-b-[3px] border-l-[3px] border-foreground">CHART_01</div>
        <h3 class="text-xl font-bold uppercase tracking-wider mb-6">Estado Global</h3>
        <div class="flex-1 relative min-h-0">
           <Doughnut :data="doughnutChartData" :options="doughnutOptions" />
        </div>
      </div>

      <!-- Monthly Revenue Bar Chart -->
      <div class="md:col-span-2 border-[3px] border-foreground p-6 bg-card relative shadow-[6px_6px_0_0_hsl(var(--foreground))] h-[400px] flex flex-col">
        <div class="absolute top-0 right-0 px-2 py-1 bg-foreground text-background font-mono text-[0.65rem] font-bold border-b-[3px] border-l-[3px] border-foreground">CHART_02</div>
        <h3 class="text-xl font-bold uppercase tracking-wider mb-6">Proyección de Ingresos (Facturación)</h3>
        <div class="flex-1 relative min-h-0">
          <Bar :data="barChartData" :options="barOptions" />
        </div>
      </div>

    </div>

    <!-- Decorative Section Below -->
    <div class="border-[3px] border-foreground p-8 bg-secondary relative mt-16 shadow-[4px_4px_0_0_hsl(var(--foreground))]">
      <div class="flex flex-col md:flex-row gap-6 items-center justify-between">
        <div>
          <h4 class="text-2xl font-black uppercase tracking-tighter mb-2">Acceso Rápido</h4>
          <p class="font-mono text-sm text-muted-foreground">Utilice los comandos para generar nuevas cuentas o gestionar clientes.</p>
        </div>
        <div class="flex gap-4 w-full md:w-auto">
          <button @click="$router.push('/invoices/new')" class="flex-1 md:flex-none bg-foreground text-background font-bold px-6 py-4 uppercase tracking-wider border-[3px] border-foreground hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[6px_6px_0_0_hsl(var(--accent))] transition-all whitespace-nowrap">
            + Nueva Cuenta
          </button>
        </div>
      </div>
      <!-- Marquee background text -->
      <div class="absolute bottom-0 left-0 w-full overflow-hidden pointer-events-none opacity-[0.03]">
        <div class="whitespace-nowrap text-[8rem] font-black leading-none -mb-8 select-none">
          SISTEMA DE FACTURACION ELECTRONICA /// SISTEMA DE FACTURACION ELECTRONICA
        </div>
      </div>
    </div>
  </div>
</template>
