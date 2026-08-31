<script setup lang="ts">
import { ArrowLeft, FileQuestion } from '@lucide/vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'

const router = useRouter()
</script>

<template>
  <section class="surface empty-state min-h-[28rem]">
    <div class="empty-state-icon"><FileQuestion class="size-5" /></div>
    <p class="font-mono text-xs font-semibold tracking-[0.14em] text-primary">ERROR 404</p>
    <h1 class="page-title mt-4 focus:outline-none" tabindex="-1">Esta página no existe</h1>
    <p class="mt-4 max-w-md text-sm leading-6 text-muted-foreground">
      La dirección puede haber cambiado o el enlace ya no está disponible.
    </p>
    <Button class="mt-6" @click="router.push('/')"><ArrowLeft /> Volver al resumen</Button>
  </section>
</template>
