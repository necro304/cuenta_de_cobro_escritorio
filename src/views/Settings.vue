<script setup lang="ts">
import { useRouter } from 'vue-router'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { useToast } from '@/components/ui/toast/use-toast'

const router = useRouter()
const { toast } = useToast()

import { version as appVersion } from '../../package.json'

const handleCheckUpdates = async () => {
  try {
    const result = await window.electronAPI.checkForUpdates() as { updateInfo?: { version: string } } | null
    // The autoUpdater events in main process will handle the dialogs if an update is found.
    // We can just show a toast if it's already updated.
    if (result && result.updateInfo && result.updateInfo.version === appVersion) {
      toast({ title: 'Al día', description: 'Ya tienes la última versión instalada.' })
    }
  } catch (error: unknown) {
    toast({ title: 'Error', description: 'No se pudo buscar actualizaciones.', variant: 'destructive' })
  }
}

const handleBackup = async () => {
  try {
    const result = await window.electronAPI.dbBackup()
    if (result.success) {
      toast({ title: 'Respaldo exitoso', description: result.message })
    } else if (result.message !== 'Operación cancelada.') {
      toast({ title: 'Error', description: result.message, variant: 'destructive' })
    }
  } catch (error: unknown) {
    toast({ title: 'Error del sistema', description: (error as Error).message, variant: 'destructive' })
  }
}

const handleRestore = async () => {
  if (!confirm('SYS.CONFIRM: ¿Restaurar base de datos? La aplicación se reiniciará para aplicar los cambios.')) return
  try {
    const result = await window.electronAPI.dbRestore()
    if (!result.success && result.message !== 'Operación cancelada.') {
      toast({ title: 'Error al restaurar', description: result.message, variant: 'destructive' })
    }
  } catch (error: unknown) {
    toast({ title: 'Error del sistema', description: (error as Error).message, variant: 'destructive' })
  }
}

const resetDatabase = async () => {
  if (!confirm('¿Estás seguro? Esta acción eliminará TODOS los datos y no se puede deshacer.')) return
  try {
    await window.electronAPI.dbRun('DELETE FROM invoice_items')
    await window.electronAPI.dbRun('DELETE FROM invoices')
    await window.electronAPI.dbRun('DELETE FROM clients')
    toast({ title: 'Datos eliminados', description: 'Todos los datos han sido borrados correctamente.' })
    router.push('/')
  } catch {
    toast({ title: 'Error', description: 'No se pudo completar la operación', variant: 'destructive' })
  }
}
</script>

<template>
  <div class="space-y-6">
    <h2 class="text-3xl font-bold tracking-tight">Configuración</h2>

    <Card class="max-w-2xl">
      <CardHeader>
        <CardTitle>Acerca de</CardTitle>
        <CardDescription>Información de la aplicación.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between p-4 border rounded-lg bg-card text-card-foreground gap-4">
          <div class="space-y-1">
            <h3 class="font-semibold text-lg flex items-center gap-2">
              Cuenta de Cobro Electrónica
            </h3>
            <p class="text-sm text-muted-foreground">Aplicación para generación de cuentas de cobro.</p>
          </div>
          <div class="flex flex-col items-start sm:items-end space-y-2">
            <div class="inline-flex items-center rounded-full border px-2.5 py-0.5 text-sm font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 border-transparent bg-primary text-primary-foreground shadow">
              v{{ appVersion }}
            </div>
            <Button variant="outline" size="sm" @click="handleCheckUpdates">
              Buscar actualizaciones
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>

    <Card class="max-w-2xl">
      <CardHeader>
        <CardTitle>Respaldo y Restauración</CardTitle>
        <CardDescription>Gestione copias de seguridad de todos sus datos.</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Button variant="outline" @click="handleBackup" class="w-full h-full min-h-[100px] py-4 flex flex-col items-center justify-center gap-2 whitespace-normal text-center">
            <span class="font-bold">Respaldar Base de Datos</span>
            <span class="text-xs text-muted-foreground font-normal">Guardar archivo de la base de datos local (.sqlite)</span>
          </Button>
          <Button variant="outline" @click="handleRestore" class="w-full h-full min-h-[100px] py-4 flex flex-col items-center justify-center gap-2 whitespace-normal text-center">
            <span class="font-bold">Restaurar Base de Datos</span>
            <span class="text-xs text-muted-foreground font-normal">Cargar un archivo .sqlite previamente respaldado</span>
          </Button>
        </div>
      </CardContent>
    </Card>

    <Card class="max-w-2xl border-destructive">
      <CardHeader>
        <CardTitle class="text-destructive">Zona de peligro</CardTitle>
        <CardDescription>Acciones irreversibles sobre los datos de la aplicación.</CardDescription>
      </CardHeader>
      <CardContent>
        <Button variant="destructive" @click="resetDatabase">
          Eliminar todos los datos
        </Button>
      </CardContent>
    </Card>
  </div>
</template>
