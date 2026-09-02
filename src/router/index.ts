import { createRouter, createWebHashHistory } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue'

const routes = [
  {
    path: '/',
    component: MainLayout,
    children: [
      { path: '', component: () => import('@/views/Dashboard.vue') },
      { path: 'invoices', component: () => import('@/views/Invoices.vue') },
      { path: 'invoices/new', component: () => import('@/views/InvoiceEditor.vue') },
      { path: 'invoices/edit/:id', component: () => import('@/views/InvoiceEditor.vue') },
      { path: 'clients', component: () => import('@/views/Clients.vue') },
      { path: 'clients/:id', component: () => import('@/views/ClientDetail.vue') },
      { path: 'profile', component: () => import('@/views/Profile.vue') },
      { path: 'settings', component: () => import('@/views/Settings.vue') },
      { path: ':pathMatch(.*)*', component: () => import('@/views/NotFound.vue') },
    ],
  },
  {
    path: '/print/:id',
    component: () => import('@/views/InvoicePrint.vue'),
  },
  {
    path: '/client-report/:id',
    component: () => import('@/views/ClientReport.vue'),
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
