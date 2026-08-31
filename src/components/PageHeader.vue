<script setup lang="ts">
defineProps<{
  title: string
  description?: string
}>()
</script>

<template>
  <header class="page-header">
    <div class="min-w-0">
      <div v-if="$slots.leading" class="mb-4">
        <slot name="leading" />
      </div>
      <h1 class="page-title focus:outline-none" tabindex="-1">{{ title }}</h1>
      <p v-if="description" class="page-description">{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="flex shrink-0 flex-wrap items-center gap-2">
      <slot name="actions" />
    </div>
  </header>
</template>
