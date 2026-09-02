<script setup lang="ts">
import { ref, watch, nextTick, onBeforeUnmount } from 'vue'
import SignaturePad from 'signature_pad'
import { Eraser } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { DEFAULT_SIGNATURE_COLOR } from '@/lib/signature'

const props = withDefaults(
  defineProps<{
    penColor?: string
  }>(),
  {
    penColor: DEFAULT_SIGNATURE_COLOR,
  },
)

const open = defineModel<boolean>('open', { required: true })

const emit = defineEmits<{
  save: [dataUrl: string]
}>()

const canvasRef = ref<HTMLCanvasElement | null>(null)
const isEmpty = ref(true)
let pad: SignaturePad | null = null

const initPad = () => {
  const canvas = canvasRef.value
  if (!canvas) return

  const ratio = Math.max(window.devicePixelRatio || 1, 1)
  canvas.width = canvas.offsetWidth * ratio
  canvas.height = canvas.offsetHeight * ratio
  canvas.getContext('2d')?.scale(ratio, ratio)

  pad = new SignaturePad(canvas, {
    penColor: props.penColor,
    minWidth: 1,
    maxWidth: 2.5,
  })
  pad.addEventListener('endStroke', () => {
    isEmpty.value = pad?.isEmpty() ?? true
  })
}

const destroyPad = () => {
  pad?.off()
  pad = null
  isEmpty.value = true
}

watch(open, async (isOpen) => {
  if (isOpen) {
    await nextTick()
    // Espera a que el contenido del dialog termine de montarse/animarse
    setTimeout(initPad, 50)
  } else {
    destroyPad()
  }
})

watch(
  () => props.penColor,
  (penColor) => {
    if (pad) pad.penColor = penColor
  },
)

onBeforeUnmount(destroyPad)

const clear = () => {
  pad?.clear()
  isEmpty.value = true
}

const save = () => {
  if (!pad || pad.isEmpty()) return
  emit('save', pad.toDataURL('image/png'))
  open.value = false
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-[560px]">
      <DialogHeader>
        <DialogTitle>Dibujar firma</DialogTitle>
        <DialogDescription>
          Firma con el mouse o el trackpad dentro del recuadro.
        </DialogDescription>
      </DialogHeader>
      <div class="rounded-md border-2 border-dashed border-foreground/30 bg-white">
        <canvas ref="canvasRef" class="h-[200px] w-full touch-none"></canvas>
      </div>
      <DialogFooter class="gap-2">
        <Button variant="outline" @click="clear">
          <Eraser class="mr-2 h-4 w-4" />
          Limpiar
        </Button>
        <Button variant="outline" @click="open = false">Cancelar</Button>
        <Button :disabled="isEmpty" @click="save">Guardar firma</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
