<script setup lang="ts">
import { AlertTriangle } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

withDefaults(
  defineProps<{
    open: boolean
    title: string
    description: string
    confirmLabel?: string
    busy?: boolean
    destructive?: boolean
  }>(),
  {
    confirmLabel: 'Confirmar',
    busy: false,
    destructive: false,
  },
)

const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: []
}>()
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-md">
      <DialogHeader class="text-left">
        <div
          class="mb-2 flex size-10 items-center justify-center rounded-lg"
          :class="
            destructive ? 'bg-destructive/10 text-destructive' : 'bg-accent text-accent-foreground'
          "
        >
          <AlertTriangle class="size-5" />
        </div>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription class="leading-6">{{ description }}</DialogDescription>
      </DialogHeader>
      <DialogFooter class="gap-2">
        <Button variant="outline" :disabled="busy" @click="emit('update:open', false)">
          Cancelar
        </Button>
        <Button
          :variant="destructive ? 'destructive' : 'default'"
          :disabled="busy"
          @click="emit('confirm')"
        >
          {{ busy ? 'Procesando...' : confirmLabel }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
