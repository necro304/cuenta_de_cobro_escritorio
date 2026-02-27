<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/toast/use-toast'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
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
import ScanningLine from '@/components/ui/animations/ScanningLine.vue'
import { Plus, Pencil, Trash2, CheckCircle2 } from 'lucide-vue-next'
import type { Profile, BankAccount } from '@/types'

const { toast } = useToast()

const profile = ref({
  name: '',
  document_id: '',
  address: '',
  phone: '',
  email: '',
  bank_info: '',
})

const bankAccounts = ref<BankAccount[]>([])
const isDialogOpen = ref(false)
const editingAccount = ref<Partial<BankAccount>>({
  bank: '',
  account_type: 'Ahorros',
  account_number: '',
})

const loadProfile = async () => {
  const data = await window.electronAPI.dbGet<Profile>('SELECT * FROM profile WHERE id = 1')
  if (data) {
    profile.value = {
      name: data.name ?? '',
      document_id: data.document_id ?? '',
      address: data.address ?? '',
      phone: data.phone ?? '',
      email: data.email ?? '',
      bank_info: data.bank_info ?? '',
    }
  }
}

const loadBankAccounts = async () => {
  const data = await window.electronAPI.dbQuery<BankAccount>('SELECT * FROM bank_accounts ORDER BY created_at DESC')
  bankAccounts.value = data
}

const saveProfile = async () => {
  if (!profile.value.name.trim()) {
    toast({ title: 'Error', description: 'El nombre es requerido', variant: 'destructive' })
    return
  }
  try {
    await window.electronAPI.dbRun(
      'UPDATE profile SET name = ?, document_id = ?, address = ?, phone = ?, email = ?, bank_info = ? WHERE id = 1',
      [profile.value.name || '', profile.value.document_id || '', profile.value.address || '', profile.value.phone || '', profile.value.email || '', profile.value.bank_info || ''],
    )
    toast({
      title: 'Perfil actualizado',
      description: 'Los datos de tu perfil han sido guardados correctamente.',
    })
  } catch {
    toast({ title: 'Error', description: 'No se pudo guardar el perfil', variant: 'destructive' })
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

const saveBankAccount = async () => {
  if (!editingAccount.value.bank || !editingAccount.value.account_number) {
    toast({ title: 'Error', description: 'Todos los campos son requeridos', variant: 'destructive' })
    return
  }

  try {
    if (editingAccount.value.id) {
      await window.electronAPI.dbRun(
        'UPDATE bank_accounts SET bank = ?, account_type = ?, account_number = ? WHERE id = ?',
        [editingAccount.value.bank || '', editingAccount.value.account_type || '', editingAccount.value.account_number || '', editingAccount.value.id]
      )
    } else {
      // If it's the first account, make it default
      const countResult = await window.electronAPI.dbGet<{ count: number }>('SELECT COUNT(*) as count FROM bank_accounts')
      const isDefault = countResult?.count === 0 ? 1 : 0
      
      await window.electronAPI.dbRun(
        'INSERT INTO bank_accounts (bank, account_type, account_number, is_default) VALUES (?, ?, ?, ?)',
        [editingAccount.value.bank || '', editingAccount.value.account_type || '', editingAccount.value.account_number || '', isDefault]
      )
    }
    
    isDialogOpen.value = false
    await loadBankAccounts()
    toast({ title: 'Éxito', description: 'Cuenta bancaria guardada' })
  } catch (error) {
    toast({ title: 'Error', description: 'No se pudo guardar la cuenta', variant: 'destructive' })
  }
}

const deleteAccount = async (id: number) => {
  if (!confirm('¿Estás seguro de eliminar esta cuenta?')) return
  
  try {
    await window.electronAPI.dbRun('DELETE FROM bank_accounts WHERE id = ?', [id])
    await loadBankAccounts()
    toast({ title: 'Éxito', description: 'Cuenta eliminada' })
  } catch (error) {
    toast({ title: 'Error', description: 'No se pudo eliminar la cuenta', variant: 'destructive' })
  }
}

const setDefaultAccount = async (id: number) => {
  try {
    await window.electronAPI.dbRun('UPDATE bank_accounts SET is_default = 0')
    await window.electronAPI.dbRun('UPDATE bank_accounts SET is_default = 1 WHERE id = ?', [id])
    await loadBankAccounts()
    toast({ title: 'Éxito', description: 'Cuenta predeterminada actualizada' })
  } catch (error) {
    toast({ title: 'Error', description: 'No se pudo actualizar la cuenta predeterminada', variant: 'destructive' })
  }
}

onMounted(() => {
  loadProfile()
  loadBankAccounts()
})
</script>

<template>
  <div class="space-y-12 pb-10">
    <!-- Header -->
    <div class="border-b-[4px] border-foreground pb-6 mb-8 flex flex-col md:flex-row md:items-end justify-between gap-6 relative overflow-hidden">
      <ScanningLine />
      <div>
        <h2 class="text-5xl md:text-7xl font-black tracking-tighter uppercase leading-none mb-3">Mi Perfil</h2>
        <div class="flex items-center gap-3">
          <div class="h-3 w-3 rounded-full bg-accent border border-foreground animate-pulse"></div>
          <p class="font-mono text-xs font-bold tracking-widest text-muted-foreground uppercase">Configuración de Identidad / SYS_PROFILE</p>
        </div>
      </div>
    </div>

    <div class="grid gap-6 md:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Información Personal</CardTitle>
          <CardDescription>
            Estos datos aparecerán en el encabezado de tus cuentas de cobro.
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="grid gap-2">
            <Label for="name">Nombre Completo</Label>
            <Input id="name" v-model="profile.name" />
          </div>
          <div class="grid gap-2">
            <Label for="doc">Documento de Identidad (C.C / NIT)</Label>
            <Input id="doc" v-model="profile.document_id" />
          </div>
          <div class="grid gap-2">
            <Label for="address">Dirección</Label>
            <Input id="address" v-model="profile.address" />
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div class="grid gap-2">
              <Label for="phone">Teléfono</Label>
              <Input id="phone" v-model="profile.phone" />
            </div>
            <div class="grid gap-2">
              <Label for="email">Email</Label>
              <Input id="email" v-model="profile.email" />
            </div>
          </div>
        </CardContent>
        <CardFooter>
          <Button @click="saveProfile">Guardar Perfil</Button>
        </CardFooter>
      </Card>

      <Card>
        <CardHeader class="flex flex-row items-center justify-between space-y-0">
          <div>
            <CardTitle>Cuentas Bancarias</CardTitle>
            <CardDescription>
              Gestiona tus cuentas para recibir pagos.
            </CardDescription>
          </div>
          <Button size="sm" @click="openAddAccount">
            <Plus class="mr-2 h-4 w-4" />
            Agregar
          </Button>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Banco</TableHead>
                <TableHead>Número</TableHead>
                <TableHead class="text-right">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow v-for="account in bankAccounts" :key="account.id">
                <TableCell>
                  <div class="font-medium">{{ account.bank }}</div>
                  <div class="text-xs text-muted-foreground">{{ account.account_type }}</div>
                </TableCell>
                <TableCell>
                  {{ account.account_number }}
                  <span v-if="account.is_default" class="ml-2 inline-flex items-center rounded-full bg-green-100 px-2.5 py-0.5 text-xs font-medium text-green-800">
                    Principal
                  </span>
                </TableCell>
                <TableCell class="text-right">
                  <div class="flex justify-end gap-2">
                    <Button variant="outline" size="icon" @click="setDefaultAccount(account.id)" title="Marcar como predeterminada">
                      <CheckCircle2 class="h-4 w-4" :class="account.is_default ? 'text-green-600' : 'text-muted-foreground'" />
                    </Button>
                    <Button variant="outline" size="icon" @click="openEditAccount(account)">
                      <Pencil class="h-4 w-4" />
                    </Button>
                    <Button variant="outline" size="icon" @click="deleteAccount(account.id)">
                      <Trash2 class="h-4 w-4 text-destructive" />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
              <TableRow v-if="bankAccounts.length === 0">
                <TableCell colspan="3" class="h-24 text-center">
                  No hay cuentas registradas.
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>

    <Dialog v-model:open="isDialogOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ editingAccount.id ? 'Editar' : 'Agregar' }} Cuenta Bancaria</DialogTitle>
          <DialogDescription>
            Ingresa los detalles de la cuenta donde recibirás los pagos.
          </DialogDescription>
        </DialogHeader>
        <div class="grid gap-4 py-4">
          <div class="grid gap-2">
            <Label for="edit-bank">Banco</Label>
            <Input id="edit-bank" v-model="editingAccount.bank" placeholder="Ej: Bancolombia, Davivienda..." />
          </div>
          <div class="grid gap-2">
            <Label for="edit-type">Tipo de Cuenta</Label>
            <Select v-model="editingAccount.account_type">
              <SelectTrigger>
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
            <Label for="edit-number">Número de Cuenta</Label>
            <Input id="edit-number" v-model="editingAccount.account_number" />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="isDialogOpen = false">Cancelar</Button>
          <Button @click="saveBankAccount">Guardar</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
