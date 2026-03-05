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
  ChevronLeft,
  ChevronRight,
} from 'lucide-vue-next'

const router = useRouter()
const activePath = computed(() => router.currentRoute.value.path)

const isCollapsed = ref(false)

const toggleSidebar = () => {
  isCollapsed.value = !isCollapsed.value
}

interface ClickBurstInstance {
  trigger: () => void
}
const burstRefs = ref<(ClickBurstInstance | null)[]>([])

const setBurstRef = (el: unknown, index: number) => {
  burstRefs.value[index] = el as ClickBurstInstance
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
    <aside
      class="border-r-[3px] border-foreground bg-card flex flex-col z-20 relative shadow-[4px_0_0_0_hsl(var(--foreground))] transition-all duration-300 ease-in-out"
      :class="isCollapsed ? 'w-24' : 'w-72'"
    >
      <!-- Toggle Button -->
      <button
        @click="toggleSidebar"
        class="absolute -right-[16px] top-8 w-8 h-8 rounded-full border-[3px] border-foreground bg-accent text-white flex items-center justify-center z-50 hover:scale-110 transition-transform shadow-[2px_2px_0_0_hsl(var(--foreground))]"
      >
        <ChevronRight v-if="isCollapsed" class="w-4 h-4 ml-0.5" />
        <ChevronLeft v-else class="w-4 h-4 pr-0.5" />
      </button>

      <!-- Brand Header -->
      <div
        class="border-b-[3px] border-foreground bg-accent relative overflow-hidden group min-h-[110px] flex flex-col justify-center transition-all duration-300 ease-in-out"
        :class="isCollapsed ? 'p-3' : 'p-6'"
      >
        <ScanningLine />
        <div
          class="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI4IiBoZWlnaHQ9IjgiPgo8cmVjdCB3aWR0aD0iOCIgaGVpZ2h0PSI4IiBmaWxsPSJ0cmFuc3BhcmVudCIvPgo8Y2lyY2xlIGN4PSIyIiBjeT0iMiIgcj0iMSIgZmlsbD0icmdiYSgwLDAsMCwwLjIpIi8+Cjwvc3ZnPg==')] opacity-50 mix-blend-multiply"
        ></div>
        <div class="relative z-10" :class="isCollapsed ? 'flex flex-col items-center' : ''">
          <h1
            v-if="!isCollapsed"
            class="text-4xl font-black uppercase tracking-tighter text-white drop-shadow-[2px_2px_0_rgba(0,0,0,1)] group-hover:scale-105 transition-transform origin-left whitespace-nowrap"
          >
            CuentaCobro
          </h1>
          <h1
            v-else
            class="text-2xl font-black uppercase tracking-tighter text-white drop-shadow-[2px_2px_0_rgba(0,0,0,1)] transition-transform text-center mt-2 group-hover:scale-105"
          >
            CC
          </h1>
          <div
            v-if="!isCollapsed"
            class="text-[0.65rem] font-mono font-bold mt-1 tracking-widest uppercase text-foreground bg-white inline-block px-1 border border-foreground whitespace-nowrap"
          >
            SYS.VER.2.0 // ELECTRON
          </div>
        </div>
      </div>

      <!-- Navigation -->
      <nav
        class="flex-1 space-y-4 bg-card overflow-y-auto overflow-x-hidden transition-all duration-300 ease-in-out"
        :class="isCollapsed ? 'p-3' : 'p-5'"
      >
        <button
          v-for="(item, index) in navItems"
          :key="item.path"
          class="w-full flex items-center justify-between py-3 text-sm font-bold uppercase tracking-wider border-[3px] transition-all outline-none focus:ring-4 focus:ring-accent relative overflow-hidden h-[54px]"
          :class="[
            activePath === item.path
              ? 'bg-foreground text-background border-foreground shadow-[4px_4px_0_0_hsl(var(--accent))] translate-x-[-2px] translate-y-[-2px]'
              : 'bg-card border-transparent hover:border-foreground hover:bg-secondary hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] text-foreground',
            isCollapsed ? 'px-0 justify-center' : 'px-4',
          ]"
          @click="navigate(item.path, index)"
          :title="isCollapsed ? item.name : undefined"
        >
          <ClickBurst :ref="(el) => setBurstRef(el, index)" />
          <div class="flex items-center gap-3" :class="isCollapsed ? 'justify-center w-full' : ''">
            <component :is="item.icon" class="h-5 w-5 shrink-0" />
            <span v-if="!isCollapsed" class="whitespace-nowrap transition-opacity duration-300">{{
              item.name
            }}</span>
          </div>
          <span v-if="!isCollapsed" class="text-[0.6rem] font-mono opacity-50 shrink-0">{{
            item.id
          }}</span>
        </button>
      </nav>

      <!-- Footer Settings -->
      <div
        class="border-t-[3px] border-foreground bg-card transition-all duration-300 ease-in-out"
        :class="isCollapsed ? 'p-3' : 'p-5'"
      >
        <button
          class="w-full flex items-center justify-between py-3 text-sm font-bold uppercase tracking-wider border-[3px] transition-all outline-none focus:ring-4 focus:ring-accent relative overflow-hidden h-[54px]"
          :class="[
            activePath === '/settings'
              ? 'bg-foreground text-background border-foreground shadow-[4px_4px_0_0_hsl(var(--accent))] translate-x-[-2px] translate-y-[-2px]'
              : 'bg-card border-transparent hover:border-foreground hover:bg-secondary hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[4px_4px_0_0_hsl(var(--foreground))] text-foreground',
            isCollapsed ? 'px-0 justify-center' : 'px-4',
          ]"
          @click="navigate('/settings', navItems.length)"
          :title="isCollapsed ? 'Config' : undefined"
        >
          <ClickBurst :ref="(el) => setBurstRef(el, navItems.length)" />
          <div class="flex items-center gap-3" :class="isCollapsed ? 'justify-center w-full' : ''">
            <Settings class="h-5 w-5 shrink-0" />
            <span v-if="!isCollapsed" class="whitespace-nowrap transition-opacity duration-300"
              >Config</span
            >
          </div>
          <span v-if="!isCollapsed" class="text-[0.6rem] font-mono opacity-50 shrink-0"
            >CFG_99</span
          >
        </button>
      </div>
    </aside>

    <!-- Main Content Area -->
    <main class="flex-1 overflow-auto p-8 md:p-12 relative z-0">
      <FloatingShapes />

      <!-- Content Container -->
      <div
        class="max-w-7xl mx-auto border-[3px] border-foreground shadow-[8px_8px_0_0_hsl(var(--foreground))] bg-card min-h-[calc(100vh-6rem)] p-8 md:p-12 relative"
      >
        <!-- Accent corner tape effect -->
        <div
          class="absolute -top-3 -left-3 w-12 h-12 bg-accent border-[3px] border-foreground rotate-12 z-10 shadow-[2px_2px_0_0_hsl(var(--foreground))]"
        ></div>

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
