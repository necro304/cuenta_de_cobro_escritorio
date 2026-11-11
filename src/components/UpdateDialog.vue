<script setup lang="ts">
import { computed } from 'vue'
import { AlertTriangle, Download, RefreshCw, RotateCcw } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { isWindowsHost, useUpdaterStore } from '@/stores/updater'

const updater = useUpdaterStore()

const formatMegabytes = (bytes: number) =>
  `${(bytes / 1_048_576).toLocaleString('es-CO', { maximumFractionDigits: 1 })} MB`

const title = computed(() => {
  switch (updater.phase) {
    case 'downloading':
      return 'Descargando actualización'
    case 'installed':
      return 'Actualización lista'
    case 'error':
      return 'No se pudo actualizar'
    default:
      return 'Actualización disponible'
  }
})

const description = computed(() => {
  switch (updater.phase) {
    case 'downloading':
      return isWindowsHost()
        ? 'Al terminar, la aplicación se cerrará para instalar la nueva versión y se abrirá de nuevo.'
        : 'No cierres la aplicación mientras se descarga la nueva versión.'
    case 'installed':
      return 'La actualización ha sido instalada. Se aplicará al reiniciar la aplicación.'
    case 'error':
      return updater.errorMessage ?? 'Ocurrió un error inesperado durante la actualización.'
    default:
      return `La versión ${updater.availableVersion ?? ''} está disponible. Tienes instalada la ${updater.currentVersion ?? ''}.`
  }
})

const progressLabel = computed(() => {
  const downloaded = formatMegabytes(updater.downloadedBytes)
  if (updater.totalBytes === null) return `${downloaded} descargados`
  return `${downloaded} de ${formatMegabytes(updater.totalBytes)} (${updater.progressPercent ?? 0}%)`
})

const onOpenChange = (open: boolean) => {
  if (!open) updater.dismiss()
}
</script>

<template>
  <Dialog :open="updater.dialogOpen" @update:open="onOpenChange">
    <DialogContent class="sm:max-w-md">
      <DialogHeader class="text-left">
        <div
          class="mb-2 flex size-10 items-center justify-center rounded-lg"
          :class="
            updater.phase === 'error'
              ? 'bg-destructive/10 text-destructive'
              : 'bg-accent text-accent-foreground'
          "
        >
          <AlertTriangle v-if="updater.phase === 'error'" class="size-5" />
          <RefreshCw
            v-else
            class="size-5"
            :class="updater.phase === 'downloading' ? 'animate-spin' : ''"
          />
        </div>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription class="leading-6">{{ description }}</DialogDescription>
      </DialogHeader>

      <div
        v-if="updater.phase === 'available' && updater.releaseNotes"
        class="max-h-48 overflow-y-auto whitespace-pre-wrap rounded-lg bg-secondary/60 p-3 text-sm text-muted-foreground"
      >
        {{ updater.releaseNotes }}
      </div>

      <p
        v-if="updater.phase === 'available' && isWindowsHost()"
        class="text-sm text-muted-foreground"
      >
        Durante la instalación la aplicación se cerrará y se abrirá de nuevo automáticamente.
      </p>

      <div v-if="updater.phase === 'downloading'" class="space-y-2">
        <div
          class="h-2 overflow-hidden rounded-full bg-secondary"
          role="progressbar"
          aria-label="Progreso de la descarga"
          aria-valuemin="0"
          aria-valuemax="100"
          :aria-valuenow="updater.progressPercent ?? undefined"
        >
          <div
            class="h-full rounded-full bg-primary transition-[width] duration-200"
            :class="updater.progressPercent === null ? 'w-1/3 animate-pulse' : ''"
            :style="
              updater.progressPercent === null
                ? undefined
                : { width: `${updater.progressPercent}%` }
            "
          ></div>
        </div>
        <p class="font-mono text-xs text-muted-foreground">{{ progressLabel }}</p>
      </div>

      <DialogFooter v-if="updater.phase !== 'downloading'" class="gap-2">
        <Button variant="outline" @click="updater.dismiss()">
          {{ updater.phase === 'error' ? 'Cerrar' : 'Más tarde' }}
        </Button>
        <Button v-if="updater.phase === 'available'" @click="updater.install()">
          <Download /> Descargar e instalar
        </Button>
        <Button v-else-if="updater.phase === 'installed'" @click="updater.restart()">
          <RotateCcw /> Reiniciar ahora
        </Button>
        <Button v-else-if="updater.phase === 'error'" @click="updater.install()">
          <RotateCcw /> Reintentar
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
