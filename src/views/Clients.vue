<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus, Pencil, Trash2 } from 'lucide-vue-next'
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
  DialogTrigger,
} from '@/components/ui/dialog'
import { useToast } from '@/components/ui/toast/use-toast'
import type { Client } from '@/types'

const { toast } = useToast()
const clients = ref<Client[]>([])
const isLoading = ref(true)

interface ClickBurstInstance {
  trigger: () => void
}
const burstRef = ref<ClickBurstInstance | null>(null)
const isDialogOpen = ref(false)
const isEditing = ref(false)
const editingId = ref<number | null>(null)
const newClient = ref({
  name: '',
  document_id: '',
  address: '',
  city: '',
  phone: '',
  email: '',
})

const loadClients = async () => {
  isLoading.value = true
  try {
    clients.value = await window.electronAPI.dbQuery<Client>('SELECT * FROM clients ORDER BY name ASC')
  } finally {
    isLoading.value = false
  }
}

const openNewDialog = () => {
  isEditing.value = false
  editingId.value = null
  newClient.value = { name: '', document_id: '', address: '', city: '', phone: '', email: '' }
  isDialogOpen.value = true
}

const editClient = (client: Client) => {
  isEditing.value = true
  editingId.value = client.id
  newClient.value = {
    name: client.name,
    document_id: client.document_id || '',
    address: client.address || '',
    city: client.city || '',
    phone: client.phone || '',
    email: client.email || '',
  }
  isDialogOpen.value = true
}

const deleteClient = async (id: number) => {
  if (confirm('SYS.CONFIRM: ¿Eliminar esta entidad? (Fallará si tiene cuentas de cobro asociadas)')) {
    try {
      await window.electronAPI.dbRun('DELETE FROM clients WHERE id = ?', [id])
      toast({ title: 'SYS_OK', description: 'Entidad eliminada con éxito.' })
      await loadClients()
    } catch {
      toast({ title: 'SYS_ERR', description: 'No se puede eliminar la entidad. Probablemente tiene cuentas de cobro asociadas.', variant: 'destructive' })
    }
  }
}

const closeDialog = () => {
  isDialogOpen.value = false
  setTimeout(() => {
    isEditing.value = false
    editingId.value = null
    newClient.value = { name: '', document_id: '', address: '', city: '', phone: '', email: '' }
  }, 200)
}

const saveClient = async () => {
  if (!newClient.value.name.trim()) {
    toast({ title: 'SYS_ERR', description: 'Nombre de entidad requerido.', variant: 'destructive' })
    return
  }
  try {
    if (isEditing.value && editingId.value) {
      await window.electronAPI.dbRun(
        'UPDATE clients SET name = ?, document_id = ?, address = ?, city = ?, phone = ?, email = ? WHERE id = ?',
        [newClient.value.name, newClient.value.document_id, newClient.value.address, newClient.value.city, newClient.value.phone, newClient.value.email, editingId.value],
      )
      toast({ title: 'SYS_OK', description: 'Entidad actualizada con éxito.' })
    } else {
      await window.electronAPI.dbRun(
        'INSERT INTO clients (name, document_id, address, city, phone, email) VALUES (?, ?, ?, ?, ?, ?)',
        [newClient.value.name, newClient.value.document_id, newClient.value.address, newClient.value.city, newClient.value.phone, newClient.value.email],
      )
      toast({ title: 'SYS_OK', description: 'Entidad registrada con éxito.' })
    }
    closeDialog()
    await loadClients()
  } catch {
    toast({ title: 'SYS_ERR', description: 'Fallo al escribir en la base de datos.', variant: 'destructive' })
  }
}

onMounted(loadClients)
</script>

<template>
  <div class="space-y-12">
    <!-- Header -->
    <div class="border-b-[4px] border-foreground pb-6 mb-8 flex flex-col md:flex-row md:items-end justify-between gap-6 relative overflow-hidden">
      <ScanningLine />
      <div>
        <h2 class="text-5xl md:text-7xl font-black tracking-tighter uppercase leading-none mb-3">Clientes</h2>
        <div class="flex items-center gap-3">
          <div class="h-3 w-3 rounded-full bg-accent border border-foreground animate-pulse"></div>
          <p class="font-mono text-xs font-bold tracking-widest text-muted-foreground uppercase">Base de Entidades / SYS_DB</p>
        </div>
      </div>

      <Dialog v-model:open="isDialogOpen" @update:open="(val) => { if (!val) closeDialog() }">
        <DialogTrigger as-child>
          <button 
            @click="() => { burstRef?.trigger(); openNewDialog() }"
            class="bg-foreground text-background font-bold px-6 py-4 uppercase tracking-wider border-[3px] border-foreground hover:-translate-y-2 hover:-translate-x-2 hover:shadow-[8px_8px_0_0_hsl(var(--accent))] transition-all flex items-center justify-center gap-3 active:translate-y-0 active:translate-x-0 active:shadow-none relative overflow-hidden" 
          >
            <ClickBurst ref="burstRef" />
            <Plus class="h-5 w-5" />
            <span>Nuevo Cliente</span>
          </button>
        </DialogTrigger>
        <DialogContent class="border-[4px] border-foreground shadow-[12px_12px_0_0_hsl(var(--foreground))] rounded-none bg-card p-0 overflow-hidden sm:max-w-[500px]">
          <DialogHeader class="bg-accent text-white p-6 border-b-[4px] border-foreground text-left m-0">
            <DialogTitle class="text-3xl font-black uppercase tracking-tighter m-0">{{ isEditing ? 'Editar Entidad' : 'Agregar Entidad' }}</DialogTitle>
            <DialogDescription class="text-white/80 font-mono text-xs uppercase tracking-widest mt-2 m-0">
              INGRESO DE DATOS / SYS_INPUT
            </DialogDescription>
          </DialogHeader>
          <div class="p-6 space-y-5 font-mono text-sm bg-card">
            <div class="space-y-2">
              <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Nombre / Razón Social *</label>
              <input v-model="newClient.name" class="w-full border-[3px] border-foreground bg-background p-3 outline-none focus:ring-4 focus:ring-accent transition-all font-sans font-bold text-lg uppercase" placeholder="EJ. ACME CORP" />
            </div>
            <div class="space-y-2">
              <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Documento / NIT</label>
              <input v-model="newClient.document_id" class="w-full border-[3px] border-foreground bg-background p-3 outline-none focus:ring-4 focus:ring-accent transition-all" />
            </div>
            <div class="space-y-2">
              <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Dirección</label>
              <input v-model="newClient.address" class="w-full border-[3px] border-foreground bg-background p-3 outline-none focus:ring-4 focus:ring-accent transition-all" />
            </div>
            <div class="space-y-2">
              <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Ciudad</label>
              <input v-model="newClient.city" class="w-full border-[3px] border-foreground bg-background p-3 outline-none focus:ring-4 focus:ring-accent transition-all" placeholder="EJ. BOGOTÁ" />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div class="space-y-2">
                <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Teléfono</label>
                <input v-model="newClient.phone" class="w-full border-[3px] border-foreground bg-background p-3 outline-none focus:ring-4 focus:ring-accent transition-all" />
              </div>
              <div class="space-y-2">
                <label class="font-bold uppercase tracking-widest text-[0.65rem] text-muted-foreground">Email</label>
                <input v-model="newClient.email" type="email" class="w-full border-[3px] border-foreground bg-background p-3 outline-none focus:ring-4 focus:ring-accent transition-all" />
              </div>
            </div>
          </div>
          <DialogFooter class="p-6 border-t-[4px] border-foreground bg-secondary sm:justify-center m-0">
            <button @click="saveClient" class="w-full bg-foreground text-background font-bold px-6 py-4 uppercase tracking-wider border-[3px] border-foreground hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[6px_6px_0_0_hsl(var(--accent))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none">
              {{ isEditing ? 'Actualizar Entidad' : 'Guardar Entidad' }}
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>

    <!-- Brutalist Table Container -->
    <div class="border-[4px] border-foreground shadow-[8px_8px_0_0_hsl(var(--foreground))] bg-card overflow-x-auto relative z-10">
      <table class="w-full text-left font-mono text-sm border-collapse">
        <thead>
          <tr class="bg-secondary border-b-[4px] border-foreground text-foreground uppercase tracking-widest text-xs">
            <th class="p-4 border-r-[3px] border-foreground">Entidad</th>
            <th class="p-4 border-r-[3px] border-foreground">DOC_ID</th>
            <th class="p-4 border-r-[3px] border-foreground">TEL</th>
            <th class="p-4 border-r-[3px] border-foreground">EMAIL_CONTACT</th>
            <th class="p-4 text-center w-32">CMD</th>
          </tr>
        </thead>
        <tbody class="font-medium">
          <tr v-if="isLoading">
            <td colspan="5" class="p-16 text-center border-b-[3px] border-foreground bg-card">
              <div class="flex flex-col items-center justify-center">
                <RetroSpinner size="64px" />
                <div class="mt-4 font-mono text-xs font-bold uppercase tracking-widest animate-pulse">Cargando base de datos...</div>
              </div>
            </td>
          </tr>
          
          <tr 
            v-else
            v-for="client in clients" 
            :key="client.id"
            class="group border-b-[3px] border-foreground hover:bg-accent/10 transition-colors last:border-b-0"
          >
            <td class="p-4 border-r-[3px] border-foreground font-sans font-black text-lg uppercase tracking-wide">
              {{ client.name }}
            </td>
            <td class="p-4 border-r-[3px] border-foreground">
              <span class="bg-foreground text-background px-2 py-1 text-xs">{{ client.document_id || 'N/A' }}</span>
            </td>
            <td class="p-4 border-r-[3px] border-foreground">{{ client.phone || 'N/A' }}</td>
            <td class="p-4 lowercase text-accent font-bold border-r-[3px] border-foreground">{{ client.email || 'N/A' }}</td>
            <td class="p-4 text-center">
              <div class="flex items-center justify-center gap-2">
                <button 
                  class="p-2 border-[2px] border-foreground bg-card hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                  @click="editClient(client)"
                  title="Editar"
                >
                  <Pencil class="h-4 w-4" />
                </button>
                <button 
                  class="p-2 border-[2px] border-foreground bg-destructive text-destructive-foreground hover:-translate-y-1 hover:-translate-x-1 hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] transition-all active:translate-y-0 active:translate-x-0 active:shadow-none"
                  @click="deleteClient(client.id)"
                  title="Eliminar"
                >
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
            </td>
          </tr>
          
          <tr v-if="!isLoading && clients.length === 0">
            <td colspan="5" class="p-16 text-center border-b-[3px] border-foreground bg-secondary relative overflow-hidden">
              <div class="absolute inset-0 opacity-10">
                <svg width="100%" height="100%" viewBox="0 0 100 100" preserveAspectRatio="none">
                  <line x1="0" y1="0" x2="100" y2="100" stroke="black" stroke-width="0.5" />
                  <line x1="100" y1="0" x2="0" y2="100" stroke="black" stroke-width="0.5" />
                </svg>
              </div>
              <div class="flex flex-col items-center justify-center text-muted-foreground relative z-10">
                <div class="font-sans font-black text-6xl mb-2 opacity-50 uppercase tracking-tighter">Vacio</div>
                <div class="text-xs tracking-widest font-mono font-bold bg-foreground text-background px-2 py-1">NO DATA FOUND // CLIENTS = 0</div>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
