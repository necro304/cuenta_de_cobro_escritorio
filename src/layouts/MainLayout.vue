<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import logoUrl from '@/assets/logo.svg'
import {
  LayoutDashboard,
  FileText,
  Users,
  UserCircle,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
} from '@lucide/vue'

const route = useRoute()
const isCollapsed = ref(false)
const mainContent = ref<HTMLElement | null>(null)

const navItems = [
  { name: 'Resumen', path: '/', icon: LayoutDashboard },
  { name: 'Cuentas', path: '/invoices', icon: FileText },
  { name: 'Clientes', path: '/clients', icon: Users },
  { name: 'Mi perfil', path: '/profile', icon: UserCircle },
]

const isActive = (path: string) =>
  path === '/' ? route.path === '/' : route.path === path || route.path.startsWith(`${path}/`)

const sidebarWidth = computed(() => (isCollapsed.value ? 'w-[76px]' : 'w-[248px]'))

watch(
  () => route.fullPath,
  async () => {
    mainContent.value?.scrollTo({ top: 0 })
    await nextTick()
    document.querySelector<HTMLElement>('#main-content h1')?.focus()
  },
)
</script>

<template>
  <div class="flex h-dvh overflow-hidden bg-background">
    <a
      href="#main-content"
      class="fixed left-4 top-4 z-50 -translate-y-20 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground shadow-lg transition-transform focus:translate-y-0"
    >
      Saltar al contenido
    </a>
    <aside
      class="relative hidden shrink-0 flex-col border-r border-border/70 bg-card/88 backdrop-blur-xl transition-[width] duration-200 md:flex"
      :class="sidebarWidth"
    >
      <div class="flex h-[76px] items-center gap-3 border-b px-4">
        <div class="brand-mark" aria-hidden="true">
          <img :src="logoUrl" alt="" />
        </div>
        <div v-if="!isCollapsed" class="min-w-0 leading-tight">
          <p class="truncate text-[15px] font-semibold tracking-[-0.02em]">CuentaCobro</p>
          <p class="mt-0.5 truncate text-xs text-muted-foreground">Gestión local</p>
        </div>
      </div>

      <nav class="flex-1 space-y-1.5 overflow-y-auto p-3" aria-label="Navegación principal">
        <RouterLink
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          class="group relative flex h-11 items-center gap-3 rounded-lg px-3 text-sm font-medium transition-[color,background-color,transform,box-shadow] active:scale-[0.98]"
          :class="
            isActive(item.path)
              ? 'bg-accent text-accent-foreground shadow-[inset_3px_0_0_hsl(var(--primary))]'
              : 'text-muted-foreground hover:bg-secondary hover:text-foreground'
          "
          :aria-current="isActive(item.path) ? 'page' : undefined"
          :title="isCollapsed ? item.name : undefined"
        >
          <component :is="item.icon" class="size-5 shrink-0" :stroke-width="1.8" />
          <span v-if="!isCollapsed" class="truncate">{{ item.name }}</span>
        </RouterLink>

        <div class="py-2">
          <div class="h-px bg-border"></div>
        </div>

        <RouterLink
          to="/invoices/new"
          class="flex h-11 items-center gap-3 rounded-lg bg-primary px-3 text-sm font-semibold text-primary-foreground shadow-sm transition-[background-color,transform] hover:bg-primary/90 active:scale-[0.98]"
          :title="isCollapsed ? 'Nueva cuenta' : undefined"
        >
          <Plus class="size-5 shrink-0" :stroke-width="2" />
          <span v-if="!isCollapsed" class="truncate">Nueva cuenta</span>
        </RouterLink>
      </nav>

      <div class="space-y-1.5 border-t p-3">
        <RouterLink
          to="/settings"
          class="flex h-11 items-center gap-3 rounded-lg px-3 text-sm font-medium transition-[color,background-color,box-shadow,transform] active:scale-[0.98]"
          :class="
            isActive('/settings')
              ? 'bg-accent text-accent-foreground shadow-[inset_3px_0_0_hsl(var(--primary))]'
              : 'text-muted-foreground hover:bg-secondary hover:text-foreground'
          "
          :aria-current="isActive('/settings') ? 'page' : undefined"
          :title="isCollapsed ? 'Configuración' : undefined"
        >
          <Settings class="size-5 shrink-0" :stroke-width="1.8" />
          <span v-if="!isCollapsed" class="truncate">Configuración</span>
        </RouterLink>
        <button
          type="button"
          class="flex h-10 w-full items-center gap-3 rounded-lg px-3 text-sm text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
          :aria-label="isCollapsed ? 'Expandir barra lateral' : 'Contraer barra lateral'"
          @click="isCollapsed = !isCollapsed"
        >
          <PanelLeftOpen v-if="isCollapsed" class="size-5 shrink-0" :stroke-width="1.8" />
          <PanelLeftClose v-else class="size-5 shrink-0" :stroke-width="1.8" />
          <span v-if="!isCollapsed">Contraer</span>
        </button>
      </div>
    </aside>

    <div class="flex min-w-0 flex-1 flex-col">
      <header
        class="flex h-16 shrink-0 items-center justify-between border-b bg-card/90 px-4 backdrop-blur-xl md:hidden"
      >
        <div class="flex items-center gap-3">
          <div class="brand-mark size-9 rounded-lg" aria-hidden="true">
            <img :src="logoUrl" alt="" />
          </div>
          <span class="font-semibold tracking-[-0.02em]">CuentaCobro</span>
        </div>
        <RouterLink
          to="/invoices/new"
          class="inline-flex h-9 items-center gap-2 rounded-lg bg-primary px-3 text-sm font-semibold text-primary-foreground"
        >
          <Plus class="size-4" /> Nueva
        </RouterLink>
      </header>

      <main id="main-content" ref="mainContent" class="min-w-0 flex-1 overflow-y-auto">
        <div class="mx-auto w-full max-w-[1440px] px-4 py-6 sm:px-6 sm:py-8 lg:px-10 lg:py-10">
          <router-view v-slot="{ Component }">
            <transition name="page" mode="out-in">
              <component :is="Component" />
            </transition>
          </router-view>
        </div>
      </main>

      <nav
        class="grid h-[68px] shrink-0 grid-cols-5 border-t bg-card/95 px-2 pb-[env(safe-area-inset-bottom)] backdrop-blur-xl md:hidden"
        aria-label="Navegación principal"
      >
        <RouterLink
          v-for="item in [...navItems, { name: 'Ajustes', path: '/settings', icon: Settings }]"
          :key="item.path"
          :to="item.path"
          class="my-1 flex min-w-0 flex-col items-center justify-center gap-1 rounded-lg text-[11px] font-medium transition-[color,background-color,transform] active:scale-[0.97]"
          :class="
            isActive(item.path) ? 'bg-accent text-accent-foreground' : 'text-muted-foreground'
          "
          :aria-current="isActive(item.path) ? 'page' : undefined"
        >
          <component :is="item.icon" class="size-5" :stroke-width="1.8" />
          <span class="truncate">{{ item.name }}</span>
        </RouterLink>
      </nav>
    </div>
  </div>
</template>

<style scoped>
.brand-mark {
  display: flex;
  width: 2.5rem;
  height: 2.5rem;
  flex: none;
  align-items: center;
  justify-content: center;
  border-radius: 0.75rem;
  box-shadow: 0 8px 22px hsl(var(--primary) / 0.2);
  overflow: hidden;
}

.brand-mark img {
  display: block;
  width: 100%;
  height: 100%;
}

.page-enter-active,
.page-leave-active {
  transition:
    opacity 140ms ease,
    transform 140ms ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.page-leave-to {
  opacity: 0;
}
</style>
