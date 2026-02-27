<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import FloatingShapes from '@/components/ui/animations/FloatingShapes.vue'
import ClickBurst from '@/components/ui/animations/ClickBurst.vue'
import ScanningLine from '@/components/ui/animations/ScanningLine.vue'
import {
  LayoutDashboard,
  FileText,
  Users,
  UserCircle,
  Settings,
} from 'lucide-vue-next'

const router = useRouter()
const activePath = computed(() => router.currentRoute.value.path)

interface ClickBurstInstance {
  trigger: () => void
}
const burstRefs = ref<(ClickBurstInstance | null)[]>([])

const setBurstRef = (el: any, index: number) => {
  burstRefs.value[index] = el
}

const navItems = [
  { name: 'Dashboard', path: '/', icon: LayoutDashboard, id: 'CMD_01' },
  { name: 'Cuentas', path: '/invoices', icon: FileText, id: 'CMD_02' },
  { name: 'Clientes', path: '/clients', icon: Users, id: 'CMD_03' },
  { name: 'Perfil', path: '/profile', icon: UserCircle, id: 'CMD_04' },
]

const navigate = (path: string, index: number) => {
  if (burstRefs.value[index]) {
    burstRefs.value[index].trigger()
  }
  setTimeout(() => {
    router.push(path)
  }, 100)
}
</script>

<template>
  <div class="flex h-screen bg-background overflow-hidden selection:bg-accent selection:text-white">
    <!-- Sidebar -->
    <aside class="w-72 border-r-[3px] border-foreground bg-card flex flex-col z-20 relative shadow-[4px_0_0_0_hsl(var(--foreground))]">
      
      <!-- Brand Header -->
      <div class="p-6 border-b-[3px] border-foreground bg-accent relative overflow-hidden group">
        <ScanningLine />
        <div class="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI4IiBoZWlnaHQ9IjgiPgo8cmVjdCB3aWR0aD0iOCIgaGVpZ2h0PSI4IiBmaWxsPSJ0cmFuc3BhcmVudCIvPgo8Y2lyY2xlIGN4PSIyIiBjeT0iMiIgcj0iMSIgZmlsbD0icmdiYSgwLDAsMCwwLjIpIi8+Cjwvc3ZnPg==')] opacity-50 mix-blend-multiply"></div>
        <div class="relative z-10">
          <h1 class="text-4xl font-black uppercase tracking-tighter text-white drop-shadow-[2px_2px_0_rgba(0,0,0,1)] group-hover:scale-105 transition-transform origin-left">CuentaCobro</h1>
          <div class="text-[0.65rem] font-mono font-bold mt-1 tracking-widest uppercase text-foreground bg-white inline-block px-1 border border-foreground">SYS.VER.2.0 // ELECTRON</div>
        </div>
      </div>

      <!-- Navigation -->
      <nav class="flex-1 p-5 space-y-4 bg-card overflow-y-auto overflow-x-hidden">
        <button
          v-for="(item, index) in navItems"
          :key="item.path"
          class="w-full flex items-center justify-between px-4 py-3 text-sm font-bold uppercase tracking-wider border-[3px] transition-all outline-none focus:ring-4 focus:ring-accent relative overflow-hidden"
          :class="[
            activePath === item.path 
              ? 'bg-foreground text-background border-foreground shadow-[4px_4px_0_0_hsl(var(--accent))] translate-x-[-2px] translate-y-[-2px]' 
              : 'bg-card border-transparent hover:border-foreground hover:bg-secondary hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] text-foreground'
          ]"
          @click="navigate(item.path, index)"
        >
          <ClickBurst :ref="(el) => setBurstRef(el, index)" />
          <div class="flex items-center gap-3">
            <component :is="item.icon" class="h-5 w-5" />
            <span>{{ item.name }}</span>
          </div>
          <span class="text-[0.6rem] font-mono opacity-50">{{ item.id }}</span>
        </button>
      </nav>

      <!-- Footer Settings -->
      <div class="p-5 border-t-[3px] border-foreground bg-card">
        <button
          class="w-full flex items-center justify-between px-4 py-3 text-sm font-bold uppercase tracking-wider border-[3px] transition-all outline-none focus:ring-4 focus:ring-accent relative overflow-hidden"
          :class="[
            activePath === '/settings'
              ? 'bg-foreground text-background border-foreground shadow-[4px_4px_0_0_hsl(var(--accent))] translate-x-[-2px] translate-y-[-2px]' 
              : 'bg-card border-transparent hover:border-foreground hover:bg-secondary hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] text-foreground'
          ]"
          @click="navigate('/settings', navItems.length)"
        >
          <ClickBurst :ref="(el) => setBurstRef(el, navItems.length)" />
          <div class="flex items-center gap-3">
            <Settings class="h-5 w-5" />
            <span>Config</span>
          </div>
          <span class="text-[0.6rem] font-mono opacity-50">CFG_99</span>
        </button>
      </div>
    </aside>

    <!-- Main Content Area -->
    <main class="flex-1 overflow-auto p-8 md:p-12 relative z-0">
      <FloatingShapes />
      
      <!-- Content Container -->
      <div class="max-w-7xl mx-auto border-[3px] border-foreground shadow-[8px_8px_0_0_hsl(var(--foreground))] bg-card min-h-[calc(100vh-6rem)] p-8 md:p-12 relative">
        <!-- Accent corner tape effect -->
        <div class="absolute -top-3 -left-3 w-12 h-12 bg-accent border-[3px] border-foreground rotate-12 z-10 shadow-[2px_2px_0_0_hsl(var(--foreground))]"></div>
        
        <router-view v-slot="{ Component }">
          <transition name="fade-slide" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </div>
    </main>
  </div>
</template>

<style scoped>
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
}
.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(10px);
}
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
