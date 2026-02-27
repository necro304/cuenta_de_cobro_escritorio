<script setup lang="ts">
// Simple animated wave pattern
</script>

<template>
  <div class="absolute inset-0 overflow-hidden pointer-events-none opacity-20">
    <svg width="100%" height="100%" viewBox="0 0 400 200" preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M0 100 Q 100 50 200 100 T 400 100"
        fill="none"
        stroke="currentColor"
        stroke-width="4"
        class="text-foreground animate-wave"
      />
      <path
        d="M0 120 Q 100 70 200 120 T 400 120"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        class="text-accent animate-wave-delayed"
      />
      <path
        d="M0 140 Q 100 90 200 140 T 400 140"
        fill="none"
        stroke="currentColor"
        stroke-width="1"
        class="text-foreground animate-wave-fast"
      />
    </svg>
  </div>
</template>

<style scoped>
.animate-wave {
  animation: wave 4s ease-in-out infinite;
  stroke-dasharray: 400;
  stroke-dashoffset: 400;
}

.animate-wave-delayed {
  animation: wave 6s ease-in-out infinite 1s;
  stroke-dasharray: 400;
  stroke-dashoffset: 400;
}

.animate-wave-fast {
  animation: wave 3s ease-in-out infinite 0.5s;
  stroke-dasharray: 400;
  stroke-dashoffset: 400;
}

@keyframes wave {
  0% { stroke-dashoffset: 400; transform: translateX(-10%); }
  50% { stroke-dashoffset: 0; transform: translateX(0); }
  100% { stroke-dashoffset: -400; transform: translateX(10%); }
}
</style>
