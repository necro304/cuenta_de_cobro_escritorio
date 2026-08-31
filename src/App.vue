<script setup lang="ts">
import { Toaster } from '@/components/ui/toast'
import { useTheme } from '@/composables/useTheme'

// Aplica el tema (sistema/claro/oscuro) desde el primer render en cualquier ruta
useTheme()
</script>

<template>
  <router-view />
  <Toaster />
</template>
