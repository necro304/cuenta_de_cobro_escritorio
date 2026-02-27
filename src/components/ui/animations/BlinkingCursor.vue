<template>
  <span class="inline-block w-[0.6em] h-[1em] bg-foreground align-middle animate-blink ml-1"></span>
</template>

<style scoped>
.animate-blink {
  animation: blink 1s step-end infinite;
}

@keyframes blink {
  from, to { opacity: 1; }
  50% { opacity: 0; }
}
</style>
