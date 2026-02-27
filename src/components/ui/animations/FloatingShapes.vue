<template>
  <div class="fixed inset-0 pointer-events-none -z-10 overflow-hidden">
    <!-- Animated SVG Rectangles -->
    <svg width="100%" height="100%" xmlns="http://www.w3.org/2000/svg">
      <rect
        width="100"
        height="100"
        class="text-accent opacity-[0.05] animate-float-slow"
        fill="currentColor"
        x="10%"
        y="20%"
        transform="rotate(45)"
      />
      <rect
        width="150"
        height="150"
        class="text-secondary opacity-[0.08] animate-float-normal"
        fill="currentColor"
        x="80%"
        y="10%"
        transform="rotate(-15)"
      />
      <circle
        r="80"
        class="text-accent opacity-[0.03] animate-pulse-slow"
        fill="currentColor"
        cx="70%"
        cy="80%"
      />
      <path
        d="M0,0 L50,0 L50,50 Z"
        class="text-foreground opacity-[0.04] animate-float-fast"
        fill="currentColor"
        transform="translate(100, 500) rotate(30)"
      />
    </svg>
  </div>
</template>

<style scoped>
.animate-float-slow {
  animation: float 20s ease-in-out infinite;
}
.animate-float-normal {
  animation: float 15s ease-in-out infinite reverse;
}
.animate-float-fast {
  animation: float 10s ease-in-out infinite;
}
.animate-pulse-slow {
  animation: pulse 8s ease-in-out infinite;
}

@keyframes float {
  0%, 100% { transform: translate(0, 0) rotate(0deg); }
  33% { transform: translate(30px, -50px) rotate(10deg); }
  66% { transform: translate(-20px, 40px) rotate(-5deg); }
}

@keyframes pulse {
  0%, 100% { transform: scale(1); opacity: 0.03; }
  50% { transform: scale(1.2); opacity: 0.06; }
}
</style>
