import { createRouter, createWebHashHistory } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue'
import Dashboard from '@/views/Dashboard.vue'
import Invoices from '@/views/Invoices.vue'
import InvoiceEditor from '@/views/InvoiceEditor.vue'
import InvoicePrint from '@/views/InvoicePrint.vue'
import Clients from '@/views/Clients.vue'
import Profile from '@/views/Profile.vue'
import Settings from '@/views/Settings.vue'

const routes = [
  {
    path: '/',
    component: MainLayout,
    children: [
      { path: '', component: Dashboard },
      { path: 'invoices', component: Invoices },
      { path: 'invoices/new', component: InvoiceEditor },
      { path: 'invoices/edit/:id', component: InvoiceEditor },
      { path: 'clients', component: Clients },
      { path: 'profile', component: Profile },
      { path: 'settings', component: Settings },
    ],
  },
  {
    path: '/print/:id',
    component: InvoicePrint,
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
