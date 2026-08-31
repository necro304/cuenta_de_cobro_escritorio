<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/toast/use-toast'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import SignaturePadDialog from '@/components/SignaturePadDialog.vue'
import { Plus, Pencil, Trash2, CheckCircle2, Upload, PenLine, RotateCcw } from '@lucide/vue'
import { useProfile } from '@/composables/useProfile'
import type { BankAccount, SignatureMode, TemplateId } from '@/types'
import PageHeader from '@/components/PageHeader.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const { toast } = useToast()
const { loadProfile: fetchProfile } = useProfile()
const isLoading = ref(true)
const loadError = ref(false)
const isSavingProfile = ref(false)
const pendingDeletion = ref<
  { kind: 'signature' } | { kind: 'account'; account: BankAccount } | null
>(null)
const isDeleting = ref(false)

const profile = ref({
  name: '',
  document_type: 'C.C',
  document_id: '',
  rut: '',
  address: '',
  phone: '',
  email: '',
  bank_info: '',
  signature_mode: 'auto' as SignatureMode,
  default_template: 'default' as TemplateId,
})

const signature = ref<string | null>(null)
const isSignatureDialogOpen = ref(false)
const signatureFileInput = ref<HTMLInputElement | null>(null)

const bankAccounts = ref<BankAccount[]>([])
const isDialogOpen = ref(false)
const editingAccount = ref<Partial<BankAccount>>({
  bank: '',
  account_type: 'Ahorros',
  account_number: '',
})

const loadProfile = async () => {
  const data = await fetchProfile()
  if (data) {
    profile.value = {
      name: data.name ?? '',
      document_type: data.document_type ?? 'C.C',
      document_id: data.document_id ?? '',
      rut: data.rut ?? '',
      address: data.address ?? '',
      phone: data.phone ?? '',
      email: data.email ?? '',
      bank_info: data.bank_info ?? '',
      signature_mode: data.signature_mode ?? 'auto',
      default_template: data.default_template ?? 'default',
    }
    signature.value = data.signature ?? null
  }
}

const validateSignatureFile = (file: File): string | null => {
  if (!['image/png', 'image/jpeg'].includes(file.type)) {
    return 'La firma debe ser una imagen PNG o JPG'
  }
  if (file.size > 1024 * 1024) {
    return 'La imagen no debe superar 1 MB'
  }
  return null
}

const saveSignature = async (dataUrl: string | null) => {
  try {
    await window.electronAPI.dbRun('UPDATE profile SET signature = ? WHERE id = 1', [dataUrl])
    signature.value = dataUrl
    toast({
      title: dataUrl ? 'Firma guardada' : 'Firma eliminada',
      description: dataUrl ? 'Tu firma se usará según el modo configurado.' : undefined,
    })
  } catch {
    toast({ title: 'Error', description: 'No se pudo guardar la firma', variant: 'destructive' })
  }
}

const handleSignatureFile = (event: Event) => {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return

  const error = validateSignatureFile(file)
  if (error) {
    toast({ title: 'Error', description: error, variant: 'destructive' })
    return
  }

  const reader = new FileReader()
  reader.onload = () => {
    if (typeof reader.result === 'string') saveSignature(reader.result)
  }
  reader.readAsDataURL(file)
}

const deleteSignature = () => {
  pendingDeletion.value = { kind: 'signature' }
}

const saveSignaturePrefs = async () => {
  try {
    await window.electronAPI.dbRun(
      'UPDATE profile SET signature_mode = ?, default_template = ? WHERE id = 1',
      [profile.value.signature_mode, profile.value.default_template],
    )
  } catch {
    toast({
      title: 'Error',
      description: 'No se pudo guardar la preferencia',
      variant: 'destructive',
    })
  }
}

const updateSignatureMode = (value: unknown) => {
  profile.value.signature_mode = value as SignatureMode
  saveSignaturePrefs()
}

const updateDefaultTemplate = (value: unknown) => {
  profile.value.default_template = value as TemplateId
  saveSignaturePrefs()
}

const loadBankAccounts = async () => {
  const data = await window.electronAPI.dbQuery<BankAccount>(
    'SELECT * FROM bank_accounts ORDER BY created_at DESC',
  )
  bankAccounts.value = data
}

const validateProfile = (): string | null => {
  if (!profile.value.name.trim()) return 'El nombre es requerido'
  return null
}

const saveProfile = async () => {
  const error = validateProfile()
  if (error) {
    toast({ title: 'Error', description: error, variant: 'destructive' })
    return
  }
  isSavingProfile.value = true
  try {
    await window.electronAPI.dbRun(
      'UPDATE profile SET name = ?, document_type = ?, document_id = ?, rut = ?, address = ?, phone = ?, email = ?, bank_info = ? WHERE id = 1',
      [
        profile.value.name || '',
        profile.value.document_type || 'C.C',
        profile.value.document_id || '',
        profile.value.rut || '',
        profile.value.address || '',
        profile.value.phone || '',
        profile.value.email || '',
        profile.value.bank_info || '',
      ],
    )
    toast({
      title: 'Perfil actualizado',
      description: 'Los datos de tu perfil han sido guardados correctamente.',
    })
  } catch {
    toast({ title: 'Error', description: 'No se pudo guardar el perfil', variant: 'destructive' })
  } finally {
    isSavingProfile.value = false
  }
}

const openAddAccount = () => {
  editingAccount.value = {
    bank: '',
    account_type: 'Ahorros',
    account_number: '',
  }
  isDialogOpen.value = true
}

const openEditAccount = (account: BankAccount) => {
  editingAccount.value = { ...account }
  isDialogOpen.value = true
}

const validateBankAccount = (): string | null => {
  if (!editingAccount.value.bank?.trim()) return 'El banco es requerido'
  if (!editingAccount.value.account_number?.trim()) return 'El número de cuenta es requerido'
  return null
}

const saveBankAccount = async () => {
  const error = validateBankAccount()
  if (error) {
    toast({ title: 'Error', description: error, variant: 'destructive' })
    return
  }

  try {
    if (editingAccount.value.id) {
      await window.electronAPI.dbRun(
        'UPDATE bank_accounts SET bank = ?, account_type = ?, account_number = ? WHERE id = ?',
        [
          editingAccount.value.bank || '',
          editingAccount.value.account_type || '',
          editingAccount.value.account_number || '',
          editingAccount.value.id,
        ],
      )
    } else {
      // If it's the first account, make it default
      const countResult = await window.electronAPI.dbGet<{ count: number }>(
        'SELECT COUNT(*) as count FROM bank_accounts',
      )
      const isDefault = countResult?.count === 0 ? 1 : 0

      await window.electronAPI.dbRun(
        'INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, ?)',
        [
          editingAccount.value.bank || '',
          editingAccount.value.account_type || '',
          editingAccount.value.account_number || '',
          isDefault,
        ],
      )
    }

    isDialogOpen.value = false
    await loadBankAccounts()
    toast({ title: 'Éxito', description: 'Cuenta bancaria guardada' })
  } catch (error) {
    toast({ title: 'Error', description: 'No se pudo guardar la cuenta', variant: 'destructive' })
  }
}

const deleteAccount = (account: BankAccount) => {
  pendingDeletion.value = { kind: 'account', account }
}

const confirmDeletion = async () => {
  const deletion = pendingDeletion.value
  if (!deletion) return

  isDeleting.value = true
  try {
    if (deletion.kind === 'signature') {
      await saveSignature(null)
    } else {
      await window.electronAPI.dbRun('DELETE FROM bank_accounts WHERE id = ?', [
        deletion.account.id,
      ])
      await loadBankAccounts()
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

const setDefaultAccount = async (id: number) => {
  try {
    await window.electronAPI.dbRun('UPDATE bank_accounts SET is_default = 0')
    await window.electronAPI.dbRun('UPDATE bank_accounts SET is_default = 1 WHERE id = ?', [id])
    await loadBankAccounts()
    toast({ title: 'Éxito', description: 'Cuenta predeterminada actualizada' })
  } catch (error) {
    toast({
      title: 'Error',
      description: 'No se pudo actualizar la cuenta predeterminada',
      variant: 'destructive',
    })
  }
}

const loadAll = async () => {
  isLoading.value = true
  loadError.value = false
  try {
    await Promise.all([loadProfile(), loadBankAccounts()])
  } catch {
    loadError.value = true
    toast({
      title: 'No se pudo cargar el perfil',
      description: 'Intenta consultar la información nuevamente.',
      variant: 'destructive',
    })
  } finally {
    isLoading.value = false
  }
}

onMounted(loadAll)
</script>

<template>
  <div class="app-page">
    <PageHeader
      title="Mi perfil"
      description="Configura los datos personales, bancarios y de firma que aparecen en tus documentos."
    />

    <div v-if="isLoading" class="grid gap-6 xl:grid-cols-12" aria-busy="true">
      <div class="surface h-[520px] animate-pulse bg-secondary/70 xl:col-span-7"></div>
      <div class="surface h-[360px] animate-pulse bg-secondary/70 xl:col-span-5"></div>
    </div>

    <div v-else-if="loadError" class="surface empty-state">
      <div class="empty-state-icon"><RotateCcw /></div>
      <h2 class="section-title">No pudimos cargar tu perfil</h2>
      <p class="mt-2 text-sm text-muted-foreground">La base de datos no respondió correctamente.</p>
      <Button class="mt-5" variant="outline" @click="loadAll"><RotateCcw /> Reintentar</Button>
    </div>

    <div v-else class="grid gap-6 xl:grid-cols-12">
      <Card class="xl:col-span-7">
        <CardHeader>
          <CardTitle>Información personal</CardTitle>
          <CardDescription>
            Estos datos aparecerán en el encabezado de tus cuentas de cobro.
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="grid gap-2">
            <Label for="name">Nombre completo</Label>
            <Input id="name" v-model="profile.name" />
          </div>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div class="grid gap-2 md:col-span-1">
              <Label for="doc-type">Tipo de documento</Label>
              <Select v-model="profile.document_type">
                <SelectTrigger id="doc-type">
                  <SelectValue placeholder="Tipo" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="C.C">C.C</SelectItem>
                  <SelectItem value="NIT">NIT</SelectItem>
                  <SelectItem value="C.E">C.E</SelectItem>
                  <SelectItem value="PPT">PPT</SelectItem>
                  <SelectItem value="Pasaporte">Pasaporte</SelectItem>
                  <SelectItem value="Otro">Otro</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="grid gap-2 md:col-span-2">
              <Label for="doc">Número de documento</Label>
              <Input id="doc" v-model="profile.document_id" />
            </div>
          </div>
          <div class="grid gap-2">
            <Label for="rut">RUT (opcional)</Label>
            <Input id="rut" v-model="profile.rut" placeholder="Ej. 123456789-2" />
          </div>
          <div class="grid gap-2">
            <Label for="address">Dirección</Label>
            <Input id="address" v-model="profile.address" />
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <div class="grid gap-2">
              <Label for="phone">Teléfono</Label>
              <Input id="phone" v-model="profile.phone" type="tel" />
            </div>
            <div class="grid gap-2">
              <Label for="email">Correo electrónico</Label>
              <Input id="email" v-model="profile.email" type="email" />
            </div>
          </div>
        </CardContent>
        <CardFooter>
          <Button :disabled="isSavingProfile" @click="saveProfile">
            {{ isSavingProfile ? 'Guardando...' : 'Guardar perfil' }}
          </Button>
        </CardFooter>
      </Card>

      <Card class="xl:col-span-5">
        <CardHeader class="flex flex-row items-center justify-between space-y-0">
          <div>
            <CardTitle>Cuentas bancarias</CardTitle>
            <CardDescription>Gestiona las cuentas donde recibes pagos.</CardDescription>
          </div>
          <Button size="sm" @click="openAddAccount">
            <Plus class="mr-2 h-4 w-4" />
            Agregar
          </Button>
        </CardHeader>
        <CardContent class="space-y-3">
          <div
            v-for="account in bankAccounts"
            :key="account.id"
            class="rounded-lg bg-secondary/55 p-4"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="flex flex-wrap items-center gap-2">
                  <p class="font-semibold">{{ account.bank }}</p>
                  <span
                    v-if="account.is_default"
                    class="inline-flex items-center rounded-md bg-[hsl(var(--success)/0.12)] px-2 py-0.5 text-xs font-semibold text-[hsl(var(--status-success))]"
                  >
                    Principal
                  </span>
                </div>
                <p class="mt-1 font-mono text-xs text-muted-foreground">
                  {{ account.account_type }} · {{ account.account_number }}
                </p>
              </div>
              <div class="flex shrink-0 gap-1.5">
                <Button
                  variant="outline"
                  size="icon-sm"
                  aria-label="Marcar como cuenta principal"
                  title="Marcar como principal"
                  @click="setDefaultAccount(account.id)"
                >
                  <CheckCircle2
                    :class="
                      account.is_default ? 'text-[hsl(var(--success))]' : 'text-muted-foreground'
                    "
                  />
                </Button>
                <Button
                  variant="outline"
                  size="icon-sm"
                  aria-label="Editar cuenta bancaria"
                  title="Editar"
                  @click="openEditAccount(account)"
                >
                  <Pencil />
                </Button>
                <Button
                  variant="outline"
                  size="icon-sm"
                  aria-label="Eliminar cuenta bancaria"
                  title="Eliminar"
                  @click="deleteAccount(account)"
                >
                  <Trash2 class="text-destructive" />
                </Button>
              </div>
            </div>
          </div>
          <div
            v-if="bankAccounts.length === 0"
            class="rounded-xl bg-secondary/50 px-4 py-10 text-center text-sm text-muted-foreground"
          >
            No hay cuentas registradas.
          </div>
        </CardContent>
      </Card>

      <Card class="xl:col-span-12">
        <CardHeader>
          <CardTitle>Firma</CardTitle>
          <CardDescription>
            Tu firma aparecerá en las cuentas de cobro según el modo que elijas.
          </CardDescription>
        </CardHeader>
        <CardContent class="grid gap-6 lg:grid-cols-2">
          <div class="space-y-4">
            <div
              class="flex h-40 items-center justify-center rounded-lg border border-stone-200 bg-white"
            >
              <img
                v-if="signature"
                :src="signature"
                alt="Firma actual"
                class="max-h-32 max-w-full object-contain p-3"
              />
              <p v-else class="text-sm text-stone-500">No has configurado una firma.</p>
            </div>

            <div class="flex flex-wrap gap-2">
              <Button variant="outline" size="sm" @click="signatureFileInput?.click()"
                ><Upload /> Subir imagen</Button
              >
              <Button variant="outline" size="sm" @click="isSignatureDialogOpen = true"
                ><PenLine /> Dibujar</Button
              >
              <Button v-if="signature" variant="outline" size="sm" @click="deleteSignature"
                ><Trash2 class="text-destructive" /> Eliminar</Button
              >
            </div>
            <input
              ref="signatureFileInput"
              type="file"
              accept="image/png,image/jpeg"
              class="hidden"
              @change="handleSignatureFile"
            />
          </div>

          <div class="space-y-4 rounded-lg bg-secondary/55 p-4">
            <div class="grid gap-2">
              <Label for="signature-mode">Uso de la firma en documentos</Label>
              <Select
                :model-value="profile.signature_mode"
                @update:model-value="updateSignatureMode"
              >
                <SelectTrigger id="signature-mode"
                  ><SelectValue placeholder="Selecciona el modo"
                /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="auto">Automática (siempre incluirla)</SelectItem>
                  <SelectItem value="none">Sin firma</SelectItem>
                  <SelectItem value="ask">Preguntar al generar</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div class="grid gap-2">
              <Label for="default-template">Plantilla predeterminada</Label>
              <Select
                :model-value="profile.default_template"
                @update:model-value="updateDefaultTemplate"
              >
                <SelectTrigger id="default-template"
                  ><SelectValue placeholder="Selecciona la plantilla"
                /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="default">Por defecto</SelectItem>
                  <SelectItem value="simple">Simple</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>

    <Dialog v-model:open="isDialogOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ editingAccount.id ? 'Editar' : 'Agregar' }} cuenta bancaria</DialogTitle>
          <DialogDescription>
            Ingresa los detalles de la cuenta donde recibirás los pagos.
          </DialogDescription>
        </DialogHeader>
        <div class="grid gap-4 py-4">
          <div class="grid gap-2">
            <Label for="edit-bank">Banco</Label>
            <Input
              id="edit-bank"
              v-model="editingAccount.bank"
              placeholder="Ej. Bancolombia o Davivienda"
            />
          </div>
          <div class="grid gap-2">
            <Label for="edit-type">Tipo de cuenta</Label>
            <Select v-model="editingAccount.account_type">
              <SelectTrigger id="edit-type">
                <SelectValue placeholder="Selecciona el tipo" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Ahorros">Ahorros</SelectItem>
                <SelectItem value="Corriente">Corriente</SelectItem>
                <SelectItem value="Nequi">Nequi</SelectItem>
                <SelectItem value="Daviplata">Daviplata</SelectItem>
                <SelectItem value="Otro">Otro</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="grid gap-2">
            <Label for="edit-number">Número de cuenta</Label>
            <Input id="edit-number" v-model="editingAccount.account_number" />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="isDialogOpen = false">Cancelar</Button>
          <Button @click="saveBankAccount">Guardar</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <SignaturePadDialog v-model:open="isSignatureDialogOpen" @save="saveSignature" />
    <ConfirmDialog
      :open="pendingDeletion !== null"
      :title="pendingDeletion?.kind === 'signature' ? 'Eliminar firma' : 'Eliminar cuenta bancaria'"
      :description="
        pendingDeletion?.kind === 'signature'
          ? 'La firma dejará de estar disponible para tus documentos. Esta acción no se puede deshacer.'
          : `Se eliminará la cuenta de ${pendingDeletion?.account.bank ?? 'este banco'}. Esta acción no se puede deshacer.`
      "
      :confirm-label="pendingDeletion?.kind === 'signature' ? 'Eliminar firma' : 'Eliminar cuenta'"
      :busy="isDeleting"
      destructive
      @update:open="(open) => !open && !isDeleting && (pendingDeletion = null)"
      @confirm="confirmDeletion"
    />
  </div>
</template>
