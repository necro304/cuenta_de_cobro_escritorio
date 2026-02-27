<script setup lang="ts">
import { ref } from 'vue';

const active = ref(false);

const trigger = () => {
  active.value = true;
  setTimeout(() => {
    active.value = false;
  }, 600);
};

defineExpose({ trigger });
</script>

<template>
  <div v-if="active" class="absolute inset-0 pointer-events-none z-50 overflow-hidden">
    <svg viewBox="0 0 100 100" class="w-full h-full">
      <g v-for="i in 8" :key="i" :style="{ transform: `rotate(${i * 45}deg)` }">
        <rect
          x="48"
          y="0"
          width="4"
          height="15"
          fill="currentColor"
          class="text-accent animate-burst-line"
        />
      </g>
    </svg>
  </div>
</template>

<style scoped>
.animate-burst-line {
  animation: burst 0.5s ease-out forwards;
}

@keyframes burst {
  0% { transform: translateY(50px) scaleY(0); opacity: 0; }
  50% { transform: translateY(20px) scaleY(1); opacity: 1; }
  100% { transform: translateY(0px) scaleY(0); opacity: 0; }
}
</style>
