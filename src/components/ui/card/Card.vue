<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <div
    :class="
      cn(
        'rounded-[1.125rem] border border-border/70 bg-card text-card-foreground shadow-[0_1px_2px_hsl(var(--foreground)/0.025),0_18px_42px_hsl(var(--primary)/0.055)]',
        props.class,
      )
    "
  >
    <slot />
  </div>
</template>
