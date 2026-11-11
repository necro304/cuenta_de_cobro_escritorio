<script setup lang="ts">
import { Toaster } from '@/components/ui/toast'
import UpdateDialog from '@/components/UpdateDialog.vue'
import { useThemeStore } from '@/stores/theme'

// Aplica el tema (sistema/claro/oscuro) desde el primer render en cualquier ruta
useThemeStore()
</script>

<template>
  <router-view />
  <Toaster />
  <UpdateDialog />
</template>
