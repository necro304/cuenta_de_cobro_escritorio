<template>
  <div class="absolute inset-0 pointer-events-none overflow-hidden opacity-20">
    <div class="w-full h-[2px] bg-accent shadow-[0_0_10px_rgba(var(--accent-rgb),0.5)] animate-scan"></div>
  </div>
</template>

<style scoped>
.animate-scan {
  animation: scan 3s linear infinite;
}

@keyframes scan {
  0% { transform: translateY(-100%); }
  100% { transform: translateY(1000%); }
}
</style>
