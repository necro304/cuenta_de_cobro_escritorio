<script setup lang="ts">
import type { ToastCloseProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import { X } from '@lucide/vue'
import { ToastClose } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = defineProps<
  ToastCloseProps & {
    class?: HTMLAttributes['class']
  }
>()

const delegatedProps = reactiveOmit(props, 'class')
</script>

<template>
  <ToastClose
    v-bind="delegatedProps"
    :class="
      cn(
        'absolute right-2 top-2 rounded-md p-1.5 text-muted-foreground transition-[color,background-color] hover:bg-secondary hover:text-foreground focus:outline-hidden focus:ring-2 focus:ring-ring/30 group-[.destructive]:text-destructive',
        props.class,
      )
    "
    aria-label="Cerrar notificación"
  >
    <X class="h-4 w-4" />
  </ToastClose>
</template>
