<script setup lang="ts">
import { PenLine, FileText } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

const open = defineModel<boolean>('open', { required: true })

const emit = defineEmits<{
  choice: [withSignature: boolean]
}>()

const choose = (withSignature: boolean) => {
  open.value = false
  emit('choice', withSignature)
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-[420px]">
      <DialogHeader>
        <DialogTitle>¿Incluir tu firma?</DialogTitle>
        <DialogDescription>
          Elige si este documento debe llevar tu firma guardada.
        </DialogDescription>
      </DialogHeader>
      <DialogFooter class="gap-2 sm:justify-center">
        <Button variant="outline" @click="choose(false)">
          <FileText class="mr-2 h-4 w-4" />
          Sin firma
        </Button>
        <Button @click="choose(true)">
          <PenLine class="mr-2 h-4 w-4" />
          Con firma
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
