<script setup lang="ts">
import { computed } from 'vue'
import { INVOICE_STATUS_LABELS } from '@/lib/invoice'
import type { Invoice } from '@/types'

const props = defineProps<{
  status: Invoice['status']
}>()

const classes = computed(() => ({
  'border-[hsl(var(--success)/0.24)] bg-[hsl(var(--success)/0.1)] text-[hsl(var(--status-success))]':
    props.status === 'paid',
  'border-[hsl(var(--info)/0.24)] bg-[hsl(var(--info)/0.1)] text-[hsl(var(--status-info))]':
    props.status === 'partially_paid',
  'border-[hsl(var(--warning)/0.26)] bg-[hsl(var(--warning)/0.1)] text-[hsl(var(--status-warning))]':
    props.status === 'draft',
}))
</script>

<template>
  <span
    class="inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-xs font-semibold"
    :class="classes"
  >
    <span class="size-1.5 rounded-[2px] bg-current opacity-75" aria-hidden="true"></span>
    {{ INVOICE_STATUS_LABELS[status] }}
  </span>
</template>
