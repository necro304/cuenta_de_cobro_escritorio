<script setup lang="ts">
import { ref, type Component } from 'vue'
import { useRouter } from 'vue-router'
import {
  Sun,
  Moon,
  Monitor,
  RefreshCw,
  DatabaseBackup,
  DatabaseZap,
  AlertTriangle,
} from '@lucide/vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/toast/use-toast'
import { getErrorMessage } from '@/lib/utils'
import { useTheme, type ThemeMode } from '@/composables/useTheme'
import PageHeader from '@/components/PageHeader.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const router = useRouter()
const { toast } = useToast()

const { theme, setTheme } = useTheme()
const activeOperation = ref<'updates' | 'backup' | 'restore' | 'reset' | null>(null)
const pendingConfirmation = ref<'restore' | 'reset' | null>(null)

const themeOptions: { value: ThemeMode; label: string; icon: Component }[] = [
  { value: 'light', label: 'Claro', icon: Sun },
  { value: 'dark', label: 'Oscuro', icon: Moon },
  { value: 'auto', label: 'Sistema', icon: Monitor },
]

import { version as appVersion } from '../../package.json'

const handleCheckUpdates = async () => {
  activeOperation.value = 'updates'
  try {
    const result = await window.electronAPI.checkForUpdates()
    // Los eventos de autoUpdater en el main process manejan los diálogos si hay
    // actualización; aquí solo avisamos cuando ya está al día.
    if (result?.updateInfo?.version === appVersion) {
      toast({ title: 'Al día', description: 'Ya tienes la última versión instalada.' })
    }
  } catch {
    toast({
      title: 'Error',
      description: 'No se pudo buscar actualizaciones.',
      variant: 'destructive',
    })
  } finally {
    activeOperation.value = null
  }
}

const handleBackup = async () => {
  activeOperation.value = 'backup'
  try {
    const result = await window.electronAPI.dbBackup()
    if (result.success) {
      toast({ title: 'Respaldo exitoso', description: result.message })
    } else if (result.message !== 'Operación cancelada.') {
      toast({ title: 'Error', description: result.message, variant: 'destructive' })
    }
  } catch (error) {
    toast({
      title: 'Error del sistema',
      description: getErrorMessage(error),
      variant: 'destructive',
    })
  } finally {
    activeOperation.value = null
  }
}

const handleRestore = () => {
  pendingConfirmation.value = 'restore'
}

const restoreDatabase = async () => {
  activeOperation.value = 'restore'
  try {
    const result = await window.electronAPI.dbRestore()
    if (!result.success && result.message !== 'Operación cancelada.') {
      toast({ title: 'Error al restaurar', description: result.message, variant: 'destructive' })
    }
  } catch (error) {
    toast({
      title: 'Error del sistema',
      description: getErrorMessage(error),
      variant: 'destructive',
    })
  } finally {
    activeOperation.value = null
  }
}

const resetDatabase = () => {
  pendingConfirmation.value = 'reset'
}

const clearDatabase = async () => {
  activeOperation.value = 'reset'
  try {
    await window.electronAPI.dbRun('DELETE FROM invoice_items')
    await window.electronAPI.dbRun('DELETE FROM invoices')
    await window.electronAPI.dbRun('DELETE FROM clients')
    toast({
      title: 'Datos eliminados',
      description: 'Todos los datos han sido borrados correctamente.',
    })
    router.push('/')
  } catch {
    toast({
      title: 'Error',
      description: 'No se pudo completar la operación',
      variant: 'destructive',
    })
  } finally {
    activeOperation.value = null
  }
}

const confirmOperation = async () => {
  const operation = pendingConfirmation.value
  if (!operation) return

  if (operation === 'restore') await restoreDatabase()
  else await clearDatabase()

  pendingConfirmation.value = null
}
</script>

<template>
  <div class="app-page">
    <PageHeader
      title="Configuración"
      description="Ajusta la apariencia, las actualizaciones y la protección de tus datos locales."
    />

    <div class="grid gap-6 lg:grid-cols-12">
      <Card class="lg:col-span-6">
        <CardHeader>
          <CardTitle>Acerca de</CardTitle>
          <CardDescription>Información de esta instalación.</CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div
            class="flex flex-col items-start justify-between gap-4 rounded-lg bg-secondary/60 p-4 sm:flex-row sm:items-center"
          >
            <div class="space-y-1">
              <h3 class="font-semibold text-lg flex items-center gap-2">
                Cuenta de Cobro Electrónica
              </h3>
              <p class="text-sm text-muted-foreground">
                Tus cuentas de cobro y respaldos permanecen en este equipo.
              </p>
            </div>
            <div class="flex flex-col items-start sm:items-end space-y-2">
              <div class="font-mono text-xs font-medium text-muted-foreground">
                v{{ appVersion }}
              </div>
              <Button
                variant="outline"
                size="sm"
                :disabled="activeOperation !== null"
                @click="handleCheckUpdates"
              >
                <RefreshCw :class="activeOperation === 'updates' ? 'animate-spin' : ''" />
                {{ activeOperation === 'updates' ? 'Buscando...' : 'Buscar actualizaciones' }}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card class="lg:col-span-6">
        <CardHeader>
          <CardTitle>Apariencia</CardTitle>
          <CardDescription>Elige cómo quieres ver la aplicación.</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="grid grid-cols-3 gap-2 rounded-lg bg-secondary/60 p-2">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              type="button"
              class="flex min-h-20 flex-col items-center justify-center gap-2 rounded-lg px-3 text-sm font-semibold transition-[background-color,color,box-shadow,transform] active:scale-[0.98]"
              :class="
                theme === option.value
                  ? 'bg-card text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              "
              :aria-pressed="theme === option.value"
              @click="setTheme(option.value)"
            >
              <component :is="option.icon" class="w-4 h-4" />
              {{ option.label }}
            </button>
          </div>
        </CardContent>
      </Card>

      <Card class="lg:col-span-12">
        <CardHeader>
          <CardTitle>Respaldo y restauración</CardTitle>
          <CardDescription>Guarda una copia o recupera una base de datos anterior.</CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <button
              type="button"
              :disabled="activeOperation !== null"
              @click="handleBackup"
              class="group flex min-h-32 w-full items-start gap-4 rounded-lg bg-secondary/55 p-5 text-left transition-[background-color,transform] hover:bg-accent/55 active:scale-[0.99] disabled:pointer-events-none disabled:opacity-50"
            >
              <span
                class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-accent text-accent-foreground"
                ><DatabaseBackup class="size-5"
              /></span>
              <span
                ><span class="block font-semibold">Crear respaldo</span
                ><span class="mt-1 block text-sm font-normal text-muted-foreground"
                  >Guarda una copia local en formato .sqlite.</span
                ></span
              >
            </button>
            <button
              type="button"
              :disabled="activeOperation !== null"
              @click="handleRestore"
              class="group flex min-h-32 w-full items-start gap-4 rounded-lg bg-secondary/55 p-5 text-left transition-[background-color,transform] hover:bg-accent/55 active:scale-[0.99] disabled:pointer-events-none disabled:opacity-50"
            >
              <span
                class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-accent text-accent-foreground"
                ><DatabaseZap class="size-5"
              /></span>
              <span
                ><span class="block font-semibold">Restaurar respaldo</span
                ><span class="mt-1 block text-sm font-normal text-muted-foreground"
                  >Carga un archivo .sqlite y reinicia la aplicación.</span
                ></span
              >
            </button>
          </div>
        </CardContent>
      </Card>

      <Card class="border-destructive/35 lg:col-span-12">
        <CardHeader>
          <div class="flex items-center gap-3">
            <div
              class="flex size-9 items-center justify-center rounded-lg bg-destructive/10 text-destructive"
            >
              <AlertTriangle class="size-5" />
            </div>
            <CardTitle class="text-destructive">Zona de peligro</CardTitle>
          </div>
          <CardDescription
            >Acciones irreversibles sobre los datos de la aplicación.</CardDescription
          >
        </CardHeader>
        <CardContent>
          <Button variant="destructive" :disabled="activeOperation !== null" @click="resetDatabase">
            {{ activeOperation === 'reset' ? 'Eliminando...' : 'Eliminar todos los datos' }}
          </Button>
        </CardContent>
      </Card>
    </div>

    <ConfirmDialog
      :open="pendingConfirmation !== null"
      :title="pendingConfirmation === 'restore' ? 'Restaurar respaldo' : 'Eliminar todos los datos'"
      :description="
        pendingConfirmation === 'restore'
          ? 'Seleccionarás una base de datos respaldada y la aplicación se reiniciará para aplicar los cambios.'
          : 'Se eliminarán permanentemente clientes, cuentas de cobro, conceptos y abonos. Esta acción no se puede deshacer.'
      "
      :confirm-label="pendingConfirmation === 'restore' ? 'Elegir respaldo' : 'Eliminar datos'"
      :busy="activeOperation !== null"
      :destructive="pendingConfirmation === 'reset'"
      @update:open="(open) => !open && activeOperation === null && (pendingConfirmation = null)"
      @confirm="confirmOperation"
    />
  </div>
</template>
