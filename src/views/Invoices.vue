<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { Plus, Trash2, Printer, Pencil, Wallet, Search, Filter, ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from 'lucide-vue-next'
import { useRouter } from 'vue-router'
import ScanningLine from '@/components/ui/animations/ScanningLine.vue'
import RetroSpinner from '@/components/ui/animations/RetroSpinner.vue'
import ClickBurst from '@/components/ui/animations/ClickBurst.vue'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useToast } from '@/components/ui/toast/use-toast'
import type { Invoice, InvoicePayment } from '@/types'

const router = useRouter()
const { toast } = useToast()
const invoices = ref<Invoice[]>([])
const isLoading = ref(true)

// Search, Filter and Pagination State
const searchQuery = ref('')
const statusFilter = ref('all')
const currentPage = ref(1)
const itemsPerPage = ref(10)

const filteredInvoices = computed(() => {
  return invoices.value.filter(invoice => {
    const matchesSearch = 
      invoice.client_name?.toLowerCase().includes(searchQuery.value.toLowerCase()) || 
      invoice.number.toString().includes(searchQuery.value)
      
    const matchesStatus = statusFilter.value === 'all' || invoice.status === statusFilter.value
    
    return matchesSearch && matchesStatus
  })
})

const totalPages = computed(() => Math.ceil(filteredInvoices.value.length / itemsPerPage.value) || 1)

const paginatedInvoices = computed(() => {
  const start = (currentPage.value - 1) * itemsPerPage.value
  const end = start + itemsPerPage.value
  return filteredInvoices.value.slice(start, end)
})

watch([searchQuery, statusFilter], () => {
  currentPage.value = 1
})

// Payments Modal State
const isPaymentsModalOpen = ref(false)
const selectedInvoice = ref<Invoice | null>(null)
const payments = ref<InvoicePayment[]>([])
const newPayment = ref({
  date: new Date().toISOString().split('T')[0],
  amount: 0,
  notes: '',
})

const balancePendiente = computed(() => {
  if (!selectedInvoice.value) return 0
  const totalPagado = payments.value.reduce((sum, p) => sum + p.amount, 0)
  return Math.max(0, selectedInvoice.value.total - totalPagado)
})

const loadInvoices = async () => {
  isLoading.value = true
  try {
    invoices.value = await window.electronAPI.dbQuery<Invoice>(`
      SELECT i.*, c.name as client_name,
      COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0) as paid_amount
      FROM invoices i
      JOIN clients c ON i.client_id = c.id
      ORDER BY i.date DESC, i.number DESC
    `)
  } finally {
    isLoading.value = false
  }
}

const openPayments = async (invoice: Invoice) => {
  selectedInvoice.value = invoice
  newPayment.value = {
    date: new Date().toISOString().split('T')[0],
    amount: Math.max(0, invoice.total - (invoice.paid_amount || 0)),
    notes: '',
  }
  await loadPayments(invoice.id)
  isPaymentsModalOpen.value = true
}

const loadPayments = async (invoiceId: number) => {
  payments.value = await window.electronAPI.dbQuery<InvoicePayment>(
    'SELECT * FROM invoice_payments WHERE invoice_id = ? ORDER BY date DESC, id DESC',
    [invoiceId]
  )
}

const checkAndUpdateInvoiceStatus = async (invoiceId: number) => {
  await loadInvoices() // Ensure we have latest paid_amount
  const invoice = invoices.value.find(i => i.id === invoiceId)
  if (!invoice) return

  const totalPaid = invoice.paid_amount || 0
  let newStatus = invoice.status

  if (totalPaid >= invoice.total) {
    newStatus = 'paid'
  } else if (totalPaid > 0) {
    newStatus = 'partially_paid'
  } else {
    newStatus = 'draft'
  }

  if (newStatus !== invoice.status) {
    await window.electronAPI.dbRun('UPDATE invoices SET status = ? WHERE id = ?', [newStatus, invoiceId])
    await loadInvoices()
  }
}

const savePayment = async () => {
  if (!selectedInvoice.value) return
  if (newPayment.value.amount <= 0) {
    toast({ title: 'SYS_ERR', description: 'El monto debe ser mayor a 0', variant: 'destructive' })
    return
  }

  try {
    await window.electronAPI.dbRun(
      'INSERT INTO invoice_payments (invoice_id, date, amount, notes) VALUES (?, ?, ?, ?)',
      [selectedInvoice.value.id, newPayment.value.date, newPayment.value.amount, newPayment.value.notes]
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

const deletePayment = async (id: number) => {
  if (!selectedInvoice.value) return
  if (confirm('SYS.CONFIRM: ¿Eliminar este abono?')) {
    await window.electronAPI.dbRun('DELETE FROM invoice_payments WHERE id = ?', [id])
    await loadPayments(selectedInvoice.value.id)
    await checkAndUpdateInvoiceStatus(selectedInvoice.value.id)
    toast({ title: 'Abono eliminado' })
  }
}

const toggleStatus = async (invoice: Invoice) => {
  let newStatus = 'draft'
  if (invoice.status === 'draft') newStatus = 'partially_paid'
  else if (invoice.status === 'partially_paid') newStatus = 'paid'
  
  try {
    await window.electronAPI.dbRun('UPDATE invoices SET status = ? WHERE id = ?', [newStatus, invoice.id])
    await loadInvoices()
  } catch (error) {
    console.error('Error updating status', error)
  }
}

const deleteInvoice = async (id: number) => {
  if (confirm('SYS.CONFIRM: ¿Eliminar este registro de forma permanente?')) {
    await window.electronAPI.dbRun('DELETE FROM invoices WHERE id = ?', [id])
    await loadInvoices()
  }
}

onMounted(loadInvoices)
</script>

<template>
  <div class="space-y-12">
    
    <!-- Header -->
    <div class="border-b-[4px] border-foreground pb-6 mb-8 flex flex-col md:flex-row md:items-end justify-between gap-6 relative overflow-hidden">
      <ScanningLine />
      <div>
        <h2 class="text-5xl md:text-7xl font-black tracking-tighter uppercase leading-none mb-3">Cuentas</h2>
        <div class="flex items-center gap-3">
          <div class="h-3 w-3 rounded-full bg-accent border border-foreground animate-pulse"></div>
          <p class="font-mono text-xs font-bold tracking-widest text-muted-foreground uppercase">Registro Histórico / SYS_DATA</p>
        </div>
      </div>
      
      <button 
        class="bg-foreground text-background font-bold px-6 py-4 uppercase tracking-wider border-[3px] border-foreground hover:-translate-y-2 hover:-translate-x-2 hover:shadow-[8px_8px_0_0_hsl(var(--accent))] transition-all flex items-center justify-center gap-3 active:translate-y-0 active:translate-x-0 active:shadow-none relative overflow-hidden" 
        @click="() => { setTimeout(() => router.push('/invoices/new'), 150) }"
      >
        <Plus class="h-5 w-5" />
        <span>Nueva Cuenta</span>
      </button>
    </div>

    <!-- Filters and Search -->
    <div class="flex flex-col md:flex-row gap-4 mb-6 relative z-20">
      <div class="relative flex-1">
        <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
          <Search class="h-4 w-4 text-muted-foreground" />
        </div>
        <input 
          v-model="searchQuery" 
          type="text" 
          placeholder="Buscar por cliente o número de cuenta..." 
          class="w-full pl-10 pr-4 py-3 border-[3px] border-foreground bg-card text-foreground font-mono text-sm focus:outline-none focus:ring-2 focus:ring-accent transition-all shadow-[4px_4px_0_0_hsl(var(--foreground))]"
        />
      </div>
      
      <div class="flex items-center gap-2 border-[3px] border-foreground bg-card px-4 py-2 shadow-[4px_4px_0_0_hsl(var(--foreground))] font-mono text-sm">
        <Filter class="h-4 w-4 text-muted-foreground" />
        <select v-model="statusFilter" class="bg-transparent border-none outline-none text-foreground font-bold uppercase tracking-widest cursor-pointer">
          <option value="all">Todos los Estados</option>
          <option value="paid">Pagadas</option>
          <option value="partially_paid">Abonadas</option>
          <option value="draft">Pendientes</option>
        </select>
      </div>
    </div>

    <!-- Brutalist Table Container -->
    <div class="border-[4px] border-foreground shadow-[8px_8px_0_0_hsl(var(--foreground))] bg-card overflow-x-auto relative z-10">
      
      <table class="w-full text-left font-mono text-sm border-collapse">
        <thead>
          <tr class="bg-secondary border-b-[4px] border-foreground text-foreground uppercase tracking-widest text-xs">
            <th class="p-4 border-r-[3px] border-foreground w-24">ID_REF</th>
            <th class="p-4 border-r-[3px] border-foreground">Fecha</th>
            <th class="p-4 border-r-[3px] border-foreground">Cliente_Entidad</th>
            <th class="p-4 border-r-[3px] border-foreground text-right">Total / Saldo</th>
            <th class="p-4 border-r-[3px] border-foreground text-center w-28">Estado</th>
            <th class="p-4 text-center w-40">CMD</th>
          </tr>
        </thead>
        <tbody class="font-medium">
          <tr v-if="isLoading">
            <td colspan="6" class="p-16 text-center border-b-[3px] border-foreground bg-card">
              <div class="flex flex-col items-center justify-center">
                <RetroSpinner size="64px" />
                <div class="mt-4 font-mono text-xs font-bold uppercase tracking-widest animate-pulse">Cargando base de datos...</div>
              </div>
            </td>
          </tr>
          
          <tr 
            v-else
            v-for="invoice in paginatedInvoices" 
            :key="invoice.id"
            class="group border-b-[3px] border-foreground hover:bg-accent/10 transition-colors last:border-b-0"
          >
            <td class="p-4 border-r-[3px] border-foreground">
              <div class="flex items-center gap-2 font-black text-lg">
                <span class="text-accent">#</span>{{ invoice.number }}
              </div>
            </td>
            <td class="p-4 border-r-[3px] border-foreground">{{ invoice.date }}</td>
            <td class="p-4 border-r-[3px] border-foreground font-sans font-bold uppercase text-base tracking-wide">{{ invoice.client_name }}</td>
            <td class="p-4 border-r-[3px] border-foreground text-right font-black text-lg">
              <div class="flex flex-col">
                <span>${{ invoice.total.toLocaleString() }}</span>
                <span 
                  class="text-xs font-mono tracking-widest uppercase mt-1" 
                  :class="(invoice.total - (invoice.paid_amount || 0)) <= 0 ? 'text-green-600' : 'text-muted-foreground'"
                >
                  Saldo: ${{ Math.max(0, invoice.total - (invoice.paid_amount || 0)).toLocaleString() }}
                </span>
              </div>
            </td>
            <td class="p-4 border-r-[3px] border-foreground text-center">
              <button 
                @click="toggleStatus(invoice)"
                class="inline-block px-3 py-1 border-[2px] border-foreground font-bold text-[0.65rem] uppercase tracking-widest shadow-[2px_2px_0_0_hsl(var(--foreground))] hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                :class="{
                  'bg-green-400 text-black': invoice.status === 'paid',
                  'bg-orange-400 text-black': invoice.status === 'partially_paid',
                  'bg-yellow-400 text-black': invoice.status === 'draft'
                }"
                title="Cambiar estado"
              >
                {{ invoice.status === 'paid' ? 'Pagada' : invoice.status === 'partially_paid' ? 'Abonada' : 'Pendiente' }}
              </button>
            </td>
            <td class="p-4 text-center">
              <div class="flex items-center justify-center gap-2">
                <button 
                  class="p-2 border-[2px] border-foreground bg-card hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                  @click="openPayments(invoice)"
                  title="Abonos"
                >
                  <Wallet class="h-4 w-4" />
                </button>
                <button 
                  class="p-2 border-[2px] border-foreground bg-card hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                  @click="router.push('/invoices/edit/' + invoice.id)"
                  title="Editar"
                >
                  <Pencil class="h-4 w-4" />
                </button>
                <button 
                  class="p-2 border-[2px] border-foreground bg-card hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                  @click="router.push('/print/' + invoice.id)"
                  title="Imprimir"
                >
                  <Printer class="h-4 w-4" />
                </button>
                <button 
                  class="p-2 border-[2px] border-foreground bg-destructive text-destructive-foreground hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                  @click="deleteInvoice(invoice.id)"
                  title="Eliminar"
                >
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
            </td>
          </tr>
          
          <tr v-if="!isLoading && filteredInvoices.length === 0">
            <td colspan="6" class="p-16 text-center border-b-[3px] border-foreground bg-secondary relative overflow-hidden">
              <div class="absolute inset-0 opacity-10">
                <svg width="100%" height="100%" viewBox="0 0 100 100" preserveAspectRatio="none">
                  <line x1="0" y1="0" x2="100" y2="100" stroke="black" stroke-width="0.5" />
                  <line x1="100" y1="0" x2="0" y2="100" stroke="black" stroke-width="0.5" />
                </svg>
              </div>
              <div class="flex flex-col items-center justify-center text-muted-foreground relative z-10">
                <div class="font-sans font-black text-6xl mb-2 opacity-50 uppercase tracking-tighter">Vacio</div>
                <div class="text-xs tracking-widest font-mono font-bold bg-foreground text-background px-2 py-1">NO DATA FOUND // INVOICES = 0</div>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    <div v-if="totalPages > 1" class="flex items-center justify-between mt-6 font-mono text-sm">
      <div class="text-muted-foreground font-bold uppercase tracking-widest text-xs hidden md:block">
        Mostrando {{ filteredInvoices.length === 0 ? 0 : ((currentPage - 1) * itemsPerPage) + 1 }} - {{ Math.min(currentPage * itemsPerPage, filteredInvoices.length) }} de {{ filteredInvoices.length }} cuentas
      </div>
      <div class="flex gap-2 w-full md:w-auto justify-center">
        <button 
          @click="currentPage = 1" 
          :disabled="currentPage === 1"
          class="p-2 border-[2px] border-foreground bg-card disabled:opacity-50 disabled:cursor-not-allowed hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
        >
          <ChevronsLeft class="h-4 w-4" />
        </button>
        <button 
          @click="currentPage--" 
          :disabled="currentPage === 1"
          class="p-2 border-[2px] border-foreground bg-card disabled:opacity-50 disabled:cursor-not-allowed hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
        >
          <ChevronLeft class="h-4 w-4" />
        </button>
        
        <div class="px-4 py-2 border-[2px] border-foreground bg-secondary font-black flex items-center justify-center min-w-[80px]">
          {{ currentPage }} / {{ totalPages }}
        </div>
        
        <button 
          @click="currentPage++" 
          :disabled="currentPage === totalPages"
          class="p-2 border-[2px] border-foreground bg-card disabled:opacity-50 disabled:cursor-not-allowed hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
        >
          <ChevronRight class="h-4 w-4" />
        </button>
        <button 
          @click="currentPage = totalPages" 
          :disabled="currentPage === totalPages"
          class="p-2 border-[2px] border-foreground bg-card disabled:opacity-50 disabled:cursor-not-allowed hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
        >
          <ChevronsRight class="h-4 w-4" />
        </button>
      </div>
    </div>

    <!-- Payments Modal -->
    <Dialog v-model:open="isPaymentsModalOpen">
      <DialogContent class="border-[4px] border-foreground shadow-[12px_12px_0_0_hsl(var(--foreground))] rounded-none bg-card p-0 overflow-hidden sm:max-w-[600px]">
        <DialogHeader class="bg-accent text-white p-6 border-b-[4px] border-foreground text-left m-0">
          <DialogTitle class="text-3xl font-black uppercase tracking-tighter m-0">Abonos</DialogTitle>
          <DialogDescription class="text-white/80 font-mono text-xs uppercase tracking-widest mt-2 m-0">
            CUENTA DE COBRO #{{ selectedInvoice?.number }} / {{ selectedInvoice?.client_name }}
          </DialogDescription>
        </DialogHeader>
        
        <div class="p-6 space-y-6 font-mono text-sm bg-card">
          <!-- Summary -->
          <div class="grid grid-cols-3 gap-4 border-[3px] border-foreground p-4 bg-secondary">
            <div>
              <p class="text-xs uppercase tracking-widest text-muted-foreground font-bold">Total</p>
              <p class="font-black text-lg">${{ selectedInvoice?.total.toLocaleString() }}</p>
            </div>
            <div>
              <p class="text-xs uppercase tracking-widest text-muted-foreground font-bold">Pagado</p>
              <p class="font-black text-lg text-green-600">${{ (selectedInvoice?.total! - balancePendiente).toLocaleString() }}</p>
            </div>
            <div>
              <p class="text-xs uppercase tracking-widest text-muted-foreground font-bold">Saldo</p>
              <p class="font-black text-lg text-destructive">${{ balancePendiente.toLocaleString() }}</p>
            </div>
          </div>

          <!-- New Payment Form -->
          <div v-if="balancePendiente > 0" class="space-y-4 border-[3px] border-foreground p-4 bg-background">
            <h4 class="font-black uppercase tracking-widest text-sm border-b-[2px] border-foreground pb-2">Registrar Abono</h4>
            <div class="grid grid-cols-2 gap-4">
              <div class="space-y-2">
                <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Fecha</label>
                <input v-model="newPayment.date" type="date" class="w-full border-[3px] border-foreground bg-background p-2 outline-none focus:ring-4 focus:ring-accent transition-all" />
              </div>
              <div class="space-y-2">
                <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Monto</label>
                <input v-model.number="newPayment.amount" type="number" min="1" class="w-full border-[3px] border-foreground bg-background p-2 outline-none focus:ring-4 focus:ring-accent transition-all font-bold" />
              </div>
            </div>
            <div class="space-y-2">
              <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Notas / Referencia</label>
              <input v-model="newPayment.notes" class="w-full border-[3px] border-foreground bg-background p-2 outline-none focus:ring-4 focus:ring-accent transition-all" placeholder="EJ: TR #12345" />
            </div>
            <button @click="savePayment" class="w-full bg-foreground text-background font-bold px-4 py-3 uppercase tracking-wider border-[3px] border-foreground hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--accent))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none mt-2">
              Guardar Abono
            </button>
          </div>

          <!-- Payments History -->
          <div class="space-y-2">
            <h4 class="font-black uppercase tracking-widest text-sm border-b-[2px] border-foreground pb-2">Historial de Abonos</h4>
            <div v-if="payments.length === 0" class="text-center py-6 text-muted-foreground text-xs uppercase tracking-widest">
              SIN ABONOS REGISTRADOS
            </div>
            <div v-else class="space-y-2 max-h-[200px] overflow-y-auto pr-2">
              <div v-for="payment in payments" :key="payment.id" class="flex justify-between items-center border-[2px] border-foreground p-3 bg-secondary">
                <div>
                  <p class="font-bold text-base">${{ payment.amount.toLocaleString() }}</p>
                  <p class="text-xs text-muted-foreground">{{ payment.date }} <span v-if="payment.notes">| {{ payment.notes }}</span></p>
                </div>
                <button 
                  @click="deletePayment(payment.id)"
                  class="p-2 border-[2px] border-foreground bg-destructive text-destructive-foreground hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                  title="Eliminar Abono"
                >
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        </div>
        
        <DialogFooter class="p-6 border-t-[4px] border-foreground bg-secondary sm:justify-center m-0">
          <button @click="isPaymentsModalOpen = false" class="w-full bg-card text-foreground font-bold px-6 py-4 uppercase tracking-wider border-[3px] border-foreground hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[6px_6px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none">
            Cerrar Panel
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
