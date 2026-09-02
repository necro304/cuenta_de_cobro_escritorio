<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  Eye,
  FileText,
  Mail,
  MapPin,
  Pencil,
  Phone,
  Plus,
  RotateCcw,
  Search,
  Trash2,
  UsersRound,
  X,
} from '@lucide/vue'
import { useRouter } from 'vue-router'
import PageHeader from '@/components/PageHeader.vue'
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
import { useClients } from '@/composables/useClients'
import type { Client } from '@/types'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { formatCurrency } from '@/lib/format'

interface ClientMetric {
  id: number
  invoice_count: number
  total_issued: number
}

const { toast } = useToast()
const router = useRouter()
const { clients, loadClients: fetchClients } = useClients()
const isLoading = ref(true)
const loadError = ref(false)
const isSaving = ref(false)
const pendingDeleteClient = ref<Client | null>(null)
const isDeleting = ref(false)
const searchQuery = ref('')
const clientMetrics = ref<Record<number, ClientMetric>>({})

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
  loadError.value = false
  try {
    const [, metrics] = await Promise.all([
      fetchClients(),
      window.electronAPI.dbQuery<ClientMetric>(`
        SELECT c.id,
          COUNT(i.id) as invoice_count,
          COALESCE(SUM(i.total), 0) as total_issued
        FROM clients c
        LEFT JOIN invoices i ON i.client_id = c.id
        GROUP BY c.id
      `),
    ])
    clientMetrics.value = Object.fromEntries(metrics.map((metric) => [metric.id, metric]))
  } catch {
    loadError.value = true
    toast({
      title: 'No se pudieron cargar los clientes',
      description: 'Intenta consultar la base de datos nuevamente.',
      variant: 'destructive',
    })
  } finally {
    isLoading.value = false
  }
}

const filteredClients = computed(() => {
  const query = searchQuery.value.trim().toLocaleLowerCase('es-CO')
  if (!query) return clients.value

  return clients.value.filter((client) =>
    [client.name, client.document_id, client.city, client.email, client.phone].some((value) =>
      value?.toLocaleLowerCase('es-CO').includes(query),
    ),
  )
})

const clientSummary = computed(() => {
  const metrics = Object.values(clientMetrics.value)
  return {
    total: clients.value.length,
    withActivity: metrics.filter((metric) => metric.invoice_count > 0).length,
    cities: new Set(
      clients.value.map((client) => client.city?.trim().toLocaleLowerCase('es-CO')).filter(Boolean),
    ).size,
    completeProfiles: clients.value.filter(
      (client) => client.document_id && client.city && client.phone && client.email,
    ).length,
    totalIssued: metrics.reduce((total, metric) => total + metric.total_issued, 0),
  }
})

const getClientMetric = (clientId: number): ClientMetric =>
  clientMetrics.value[clientId] ?? { id: clientId, invoice_count: 0, total_issued: 0 }

const getInitials = (name: string) =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase())
    .join('')

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

const deleteClient = (client: Client) => {
  pendingDeleteClient.value = client
}

const confirmDeleteClient = async () => {
  const client = pendingDeleteClient.value
  if (!client) return

  isDeleting.value = true
  try {
    await window.electronAPI.dbRun('DELETE FROM clients WHERE id = ?', [client.id])
    toast({ title: 'Cliente eliminado', description: 'El registro se eliminó correctamente.' })
    await loadClients()
    pendingDeleteClient.value = null
  } catch {
    toast({
      title: 'No se pudo eliminar el cliente',
      description: 'El cliente puede tener cuentas de cobro asociadas.',
      variant: 'destructive',
    })
  } finally {
    isDeleting.value = false
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

const validate = (): string | null => {
  if (!newClient.value.name.trim()) return 'Nombre de entidad requerido.'
  return null
}

const saveClient = async () => {
  const error = validate()
  if (error) {
    toast({ title: 'Revisa los datos', description: error, variant: 'destructive' })
    return
  }
  isSaving.value = true
  try {
    if (isEditing.value && editingId.value) {
      await window.electronAPI.dbRun(
        'UPDATE clients SET name = ?, document_id = ?, address = ?, city = ?, phone = ?, email = ? WHERE id = ?',
        [
          newClient.value.name,
          newClient.value.document_id,
          newClient.value.address,
          newClient.value.city,
          newClient.value.phone,
          newClient.value.email,
          editingId.value,
        ],
      )
      toast({
        title: 'Cliente actualizado',
        description: 'Los cambios se guardaron correctamente.',
      })
    } else {
      await window.electronAPI.dbRun(
        'INSERT INTO clients (name, document_id, address, city, phone, email) VALUES (?, ?, ?, ?, ?, ?)',
        [
          newClient.value.name,
          newClient.value.document_id,
          newClient.value.address,
          newClient.value.city,
          newClient.value.phone,
          newClient.value.email,
        ],
      )
      toast({
        title: 'Cliente registrado',
        description: 'El cliente ya está disponible para tus cuentas.',
      })
    }
    closeDialog()
    await loadClients()
  } catch {
    toast({
      title: 'No se pudo guardar el cliente',
      description: 'La base de datos no respondió correctamente.',
      variant: 'destructive',
    })
  } finally {
    isSaving.value = false
  }
}

onMounted(loadClients)
</script>

<template>
  <div class="app-page">
    <PageHeader
      title="Directorio comercial"
      description="Conoce el valor de cada relación y mantén listos los datos para emitir nuevas cuentas."
    >
      <template #leading>
        <p class="font-mono text-[10px] font-semibold tracking-[0.16em] text-primary">
          BASE DE CLIENTES · {{ clientSummary.total }} REGISTROS
        </p>
      </template>
      <template #actions>
        <Button @click="openNewDialog"><Plus /> Nuevo cliente</Button>
      </template>
    </PageHeader>

    <section
      v-if="!loadError"
      class="client-overview overflow-hidden rounded-[1.5rem] border border-primary/15 bg-card shadow-[0_24px_70px_hsl(var(--primary)/0.08)]"
    >
      <div class="grid xl:grid-cols-12">
        <div
          class="relative isolate overflow-hidden bg-primary p-6 text-primary-foreground sm:p-8 xl:col-span-5"
        >
          <div class="directory-orbit" aria-hidden="true"></div>
          <div class="relative z-10 flex min-h-48 flex-col justify-between gap-10">
            <div class="flex items-center justify-between gap-4">
              <p class="flex items-center gap-2 text-sm font-medium text-primary-foreground/70">
                <UsersRound class="size-4" :stroke-width="1.8" /> Base comercial
              </p>
              <span
                class="rounded-md border border-primary-foreground/15 bg-primary-foreground/10 px-2 py-1 font-mono text-[10px] font-semibold tracking-[0.12em]"
              >
                LOCAL
              </span>
            </div>
            <div>
              <div class="flex items-end gap-3">
                <p class="metric-value text-[clamp(3.5rem,7vw,5.5rem)] font-semibold leading-[0.8]">
                  {{ clientSummary.total }}
                </p>
                <p class="pb-1 text-sm text-primary-foreground/65">clientes registrados</p>
              </div>
              <div class="mt-6 border-t border-primary-foreground/15 pt-4">
                <p class="text-xs text-primary-foreground/60">Valor histórico emitido</p>
                <p class="metric-value mt-1 text-xl font-semibold">
                  ${{ formatCurrency(clientSummary.totalIssued) }}
                </p>
              </div>
            </div>
          </div>
        </div>

        <div class="grid sm:grid-cols-3 xl:col-span-7">
          <div
            class="flex min-h-36 flex-col justify-between border-b p-5 sm:border-b-0 sm:border-r sm:p-6"
          >
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Con actividad</p>
              <FileText class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <div>
              <p class="metric-value text-3xl font-semibold">{{ clientSummary.withActivity }}</p>
              <p class="mt-2 text-xs text-muted-foreground">con cuentas emitidas</p>
            </div>
          </div>
          <div
            class="flex min-h-36 flex-col justify-between border-b p-5 sm:border-b-0 sm:border-r sm:p-6"
          >
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Cobertura</p>
              <MapPin class="size-5 text-primary" :stroke-width="1.8" />
            </div>
            <div>
              <p class="metric-value text-3xl font-semibold">{{ clientSummary.cities }}</p>
              <p class="mt-2 text-xs text-muted-foreground">ciudades registradas</p>
            </div>
          </div>
          <div class="flex min-h-36 flex-col justify-between p-5 sm:p-6">
            <div class="flex items-center justify-between gap-3">
              <p class="text-sm font-medium text-muted-foreground">Datos completos</p>
              <UsersRound class="size-5 text-[hsl(var(--success))]" :stroke-width="1.8" />
            </div>
            <div>
              <p class="metric-value text-3xl font-semibold">
                {{ clientSummary.completeProfiles }}
              </p>
              <p class="mt-2 text-xs text-muted-foreground">listos para contactar</p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section v-if="!loadError" class="surface overflow-hidden">
      <div class="flex flex-col gap-3 p-3 sm:flex-row sm:items-center">
        <label class="relative min-w-0 flex-1">
          <span class="sr-only">Buscar clientes</span>
          <Search
            class="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
          />
          <input
            v-model="searchQuery"
            type="search"
            placeholder="Buscar por nombre, NIT, ciudad o contacto"
            class="form-control border-0 bg-secondary/65 pl-10 pr-10 shadow-none focus:bg-background"
          />
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
        <div
          class="flex items-center justify-between gap-4 px-1 text-xs text-muted-foreground sm:justify-end sm:px-3"
        >
          <p>
            <span class="font-semibold text-foreground">{{ filteredClients.length }}</span>
            resultados
          </p>
          <p v-if="searchQuery" class="font-mono">de {{ clients.length }}</p>
        </div>
      </div>
    </section>

    <Dialog
      v-model:open="isDialogOpen"
      @update:open="
        (val) => {
          if (!val) closeDialog()
        }
      "
    >
      <DialogContent class="sm:max-w-[540px]">
        <DialogHeader class="text-left">
          <DialogTitle class="text-2xl">{{
            isEditing ? 'Editar cliente' : 'Nuevo cliente'
          }}</DialogTitle>
          <DialogDescription>
            {{
              isEditing
                ? 'Actualiza la información del cliente.'
                : 'Registra los datos necesarios para emitir documentos.'
            }}
          </DialogDescription>
        </DialogHeader>
        <div class="space-y-4 text-sm">
          <div class="space-y-2">
            <label for="client-name" class="field-label">Nombre o razón social</label>
            <input
              id="client-name"
              v-model="newClient.name"
              class="form-control"
              placeholder="Ej. Taller Norte SAS"
            />
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <div class="space-y-2">
              <label for="client-document" class="field-label">Documento o NIT</label>
              <input id="client-document" v-model="newClient.document_id" class="form-control" />
            </div>
            <div class="space-y-2">
              <label for="client-city" class="field-label">Ciudad</label>
              <input
                id="client-city"
                v-model="newClient.city"
                class="form-control"
                placeholder="Ej. Bogotá"
              />
            </div>
          </div>
          <div class="space-y-2">
            <label for="client-address" class="field-label">Dirección</label>
            <input id="client-address" v-model="newClient.address" class="form-control" />
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <div class="space-y-2">
              <label for="client-phone" class="field-label">Teléfono</label>
              <input id="client-phone" v-model="newClient.phone" class="form-control" />
            </div>
            <div class="space-y-2">
              <label for="client-email" class="field-label">Correo electrónico</label>
              <input
                id="client-email"
                v-model="newClient.email"
                type="email"
                class="form-control"
              />
            </div>
          </div>
        </div>
        <DialogFooter class="gap-2">
          <Button variant="outline" @click="closeDialog">Cancelar</Button>
          <Button :disabled="isSaving" @click="saveClient">
            {{ isSaving ? 'Guardando...' : isEditing ? 'Actualizar cliente' : 'Guardar cliente' }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <div v-if="loadError" class="surface empty-state">
      <div class="empty-state-icon"><RotateCcw class="size-5" /></div>
      <h2 class="section-title">No pudimos cargar los clientes</h2>
      <p class="mt-2 text-sm text-muted-foreground">La base de datos no respondió correctamente.</p>
      <Button class="mt-5" variant="outline" @click="loadClients"><RotateCcw /> Reintentar</Button>
    </div>

    <template v-else>
      <div class="surface hidden overflow-hidden md:block">
        <div class="flex items-center justify-between border-b px-5 py-4">
          <div>
            <h2 class="section-title">Relaciones comerciales</h2>
            <p class="mt-1 text-xs text-muted-foreground">
              Contacto, ubicación y actividad histórica por cliente.
            </p>
          </div>
          <p class="font-mono text-[10px] font-semibold tracking-[0.12em] text-muted-foreground">
            DIRECTORIO · COP
          </p>
        </div>
        <div class="overflow-x-auto">
          <table class="data-table min-w-[900px]">
            <thead>
              <tr>
                <th>Cliente</th>
                <th>Contacto</th>
                <th>Ubicación</th>
                <th class="w-56">Actividad</th>
                <th class="w-36 text-right">Acciones</th>
              </tr>
            </thead>
            <tbody>
              <template v-if="isLoading">
                <tr v-for="row in 5" :key="row" aria-hidden="true">
                  <td v-for="column in 5" :key="column">
                    <div class="h-5 animate-pulse rounded bg-secondary"></div>
                  </td>
                </tr>
              </template>
              <tr v-for="client in isLoading ? [] : filteredClients" :key="client.id" class="group">
                <td>
                  <div class="flex items-center gap-3">
                    <div
                      class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-accent font-mono text-xs font-semibold text-primary"
                    >
                      {{ getInitials(client.name) }}
                    </div>
                    <div class="min-w-0">
                      <p class="max-w-72 truncate font-semibold">{{ client.name }}</p>
                      <p class="mt-1 font-mono text-[11px] text-muted-foreground">
                        {{ client.document_id || 'Documento sin registrar' }}
                      </p>
                    </div>
                  </div>
                </td>
                <td>
                  <p class="flex items-center gap-2 text-sm">
                    <Mail class="size-3.5 text-muted-foreground" />
                    <span class="max-w-52 truncate">{{
                      client.email || 'Correo sin registrar'
                    }}</span>
                  </p>
                  <p class="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
                    <Phone class="size-3.5" /> {{ client.phone || 'Teléfono sin registrar' }}
                  </p>
                </td>
                <td>
                  <p class="flex items-center gap-2 font-medium">
                    <MapPin class="size-4 text-primary" /> {{ client.city || 'Sin ciudad' }}
                  </p>
                  <p class="mt-1 max-w-56 truncate text-xs text-muted-foreground">
                    {{ client.address || 'Dirección sin registrar' }}
                  </p>
                </td>
                <td>
                  <div v-if="getClientMetric(client.id).invoice_count > 0">
                    <p class="metric-value font-semibold">
                      ${{ formatCurrency(getClientMetric(client.id).total_issued) }}
                    </p>
                    <p class="mt-1 text-xs text-muted-foreground">
                      {{ getClientMetric(client.id).invoice_count }}
                      {{
                        getClientMetric(client.id).invoice_count === 1
                          ? 'cuenta emitida'
                          : 'cuentas emitidas'
                      }}
                    </p>
                  </div>
                  <div
                    v-else
                    class="inline-flex items-center gap-2 rounded-md bg-secondary px-2.5 py-1.5 text-xs text-muted-foreground"
                  >
                    <span class="size-1.5 rounded-full bg-muted-foreground/50"></span>
                    Sin actividad
                  </div>
                </td>
                <td>
                  <div class="flex justify-end gap-1.5">
                    <button
                      class="icon-button border-primary/20 bg-accent/55 text-primary hover:bg-accent"
                      type="button"
                      :aria-label="`Ver detalle de ${client.name}`"
                      title="Ver detalle"
                      @click="router.push(`/clients/${client.id}`)"
                    >
                      <Eye />
                    </button>
                    <button
                      class="icon-button"
                      type="button"
                      :aria-label="`Editar cliente ${client.name}`"
                      title="Editar"
                      @click="editClient(client)"
                    >
                      <Pencil />
                    </button>
                    <button
                      class="icon-button hover:bg-destructive/10 hover:text-destructive"
                      type="button"
                      :aria-label="`Eliminar cliente ${client.name}`"
                      title="Eliminar"
                      @click="deleteClient(client)"
                    >
                      <Trash2 />
                    </button>
                  </div>
                </td>
              </tr>
              <tr v-if="!isLoading && filteredClients.length === 0">
                <td colspan="5" class="p-0">
                  <div class="empty-state">
                    <div class="empty-state-icon"><UsersRound class="size-5" /></div>
                    <h2 class="section-title">
                      {{ searchQuery ? 'No hay coincidencias' : 'Aún no tienes clientes' }}
                    </h2>
                    <p class="mt-2 max-w-md text-sm text-muted-foreground">
                      {{
                        searchQuery
                          ? 'Prueba con otro nombre, documento o ciudad.'
                          : 'Registra un cliente para poder crear su primera cuenta de cobro.'
                      }}
                    </p>
                    <Button
                      v-if="searchQuery"
                      class="mt-5"
                      variant="outline"
                      @click="searchQuery = ''"
                      >Limpiar búsqueda</Button
                    >
                    <Button v-else class="mt-5" @click="openNewDialog"
                      ><Plus /> Nuevo cliente</Button
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
            class="surface h-44 animate-pulse bg-secondary/70"
            aria-hidden="true"
          ></div>
        </template>

        <article
          v-for="client in isLoading ? [] : filteredClients"
          :key="client.id"
          class="surface relative overflow-hidden"
        >
          <div class="p-5">
            <div class="flex items-start justify-between gap-4">
              <div class="flex min-w-0 items-center gap-3">
                <div
                  class="flex size-11 shrink-0 items-center justify-center rounded-xl bg-accent font-mono text-xs font-semibold text-primary"
                >
                  {{ getInitials(client.name) }}
                </div>
                <div class="min-w-0">
                  <h2 class="truncate text-base font-semibold">{{ client.name }}</h2>
                  <p class="mt-1 font-mono text-xs text-muted-foreground">
                    {{ client.document_id || 'Documento sin registrar' }}
                  </p>
                </div>
              </div>
              <span
                class="flex shrink-0 items-center gap-1 rounded-md bg-secondary px-2 py-1 text-xs text-muted-foreground"
              >
                <MapPin class="size-3" /> {{ client.city || 'Sin ciudad' }}
              </span>
            </div>

            <div class="mt-5 rounded-xl bg-secondary/55 p-4">
              <div class="flex items-end justify-between gap-4 border-b pb-3">
                <div>
                  <p class="text-[10px] font-semibold tracking-[0.08em] text-muted-foreground">
                    ACTIVIDAD
                  </p>
                  <p class="metric-value mt-1 text-lg font-semibold">
                    ${{ formatCurrency(getClientMetric(client.id).total_issued) }}
                  </p>
                </div>
                <p class="text-right text-xs text-muted-foreground">
                  {{ getClientMetric(client.id).invoice_count }}
                  {{ getClientMetric(client.id).invoice_count === 1 ? 'cuenta' : 'cuentas' }}
                </p>
              </div>
              <div class="mt-3 grid gap-2 text-sm">
                <p class="flex min-w-0 items-center gap-2">
                  <Mail class="size-4 shrink-0 text-muted-foreground" />
                  <span class="truncate">{{ client.email || 'Correo sin registrar' }}</span>
                </p>
                <p class="flex items-center gap-2">
                  <Phone class="size-4 text-muted-foreground" />
                  {{ client.phone || 'Teléfono sin registrar' }}
                </p>
              </div>
            </div>
          </div>

          <div class="grid grid-cols-3 border-t border-border/70 bg-secondary/25 p-2">
            <button
              type="button"
              class="flex min-h-11 items-center justify-center gap-2 rounded-lg bg-accent/50 text-sm font-semibold text-primary transition-colors hover:bg-accent"
              :aria-label="`Ver detalle de ${client.name}`"
              @click="router.push(`/clients/${client.id}`)"
            >
              <Eye class="size-4" /> Detalle
            </button>
            <button
              type="button"
              class="flex min-h-11 items-center justify-center gap-2 rounded-lg text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-primary"
              :aria-label="`Editar cliente ${client.name}`"
              @click="editClient(client)"
            >
              <Pencil class="size-4" /> Editar
            </button>
            <button
              type="button"
              class="flex min-h-11 items-center justify-center gap-2 rounded-lg text-sm font-medium text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
              :aria-label="`Eliminar cliente ${client.name}`"
              @click="deleteClient(client)"
            >
              <Trash2 class="size-4" /> Eliminar
            </button>
          </div>
        </article>

        <div v-if="!isLoading && filteredClients.length === 0" class="surface empty-state">
          <div class="empty-state-icon"><UsersRound class="size-5" /></div>
          <h2 class="section-title">
            {{ searchQuery ? 'No hay coincidencias' : 'Aún no tienes clientes' }}
          </h2>
          <p class="mt-2 max-w-md text-sm text-muted-foreground">
            {{
              searchQuery
                ? 'Prueba con otro nombre, documento o ciudad.'
                : 'Registra un cliente para poder crear su primera cuenta de cobro.'
            }}
          </p>
          <Button v-if="searchQuery" class="mt-5" variant="outline" @click="searchQuery = ''"
            >Limpiar búsqueda</Button
          >
          <Button v-else class="mt-5" @click="openNewDialog"><Plus /> Nuevo cliente</Button>
        </div>
      </div>
    </template>

    <ConfirmDialog
      :open="pendingDeleteClient !== null"
      title="Eliminar cliente"
      :description="`Se eliminará ${pendingDeleteClient?.name ?? 'este cliente'}. Si tiene cuentas asociadas, la operación será rechazada.`"
      confirm-label="Eliminar cliente"
      :busy="isDeleting"
      destructive
      @update:open="(open) => !open && !isDeleting && (pendingDeleteClient = null)"
      @confirm="confirmDeleteClient"
    />
  </div>
</template>

<style scoped>
.client-overview {
  animation: directory-enter 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
}

.directory-orbit {
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

@keyframes directory-enter {
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
