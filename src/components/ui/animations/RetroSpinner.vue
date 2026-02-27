<script setup lang="ts">
defineProps<{
  size?: string;
  color?: string;
}>();
</script>

<template>
  <div class="relative inline-flex items-center justify-center" :style="{ width: size || '40px', height: size || '40px' }">
    <svg
      viewBox="0 0 100 100"
      class="w-full h-full animate-[spin_2s_linear_infinite]"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle
        cx="50"
        cy="50"
        r="40"
        stroke="currentColor"
        stroke-width="12"
        stroke-dasharray="60 180"
        stroke-linecap="square"
        class="text-foreground"
      />
      <circle
        cx="50"
        cy="50"
        r="25"
        stroke="currentColor"
        stroke-width="8"
        stroke-dasharray="40 120"
        stroke-linecap="square"
        class="text-accent animate-[spin_1.5s_linear_infinite_reverse]"
      />
    </svg>
    <div class="absolute inset-0 border-[2px] border-foreground opacity-20"></div>
  </div>
</template>

<style scoped>
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
