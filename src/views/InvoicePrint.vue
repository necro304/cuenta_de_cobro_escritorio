<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Printer, ArrowLeft } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import type { Profile, Client, Invoice, InvoiceItem, InvoicePayment, BankAccount } from '@/types'

const route = useRoute()
const router = useRouter()
const invoiceId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id

const profile = ref<Partial<Profile>>({})
const client = ref<Partial<Client>>({})
const invoice = ref<Partial<Invoice>>({})
const items = ref<InvoiceItem[]>([])
const payments = ref<InvoicePayment[]>([])
const bankAccount = ref<BankAccount | null>(null)

const loadData = async () => {
  const profileData = await window.electronAPI.dbGet<Profile>('SELECT * FROM profile WHERE id = 1')
  if (profileData) profile.value = profileData

  const invoiceData = await window.electronAPI.dbGet<Invoice>(
    'SELECT * FROM invoices WHERE id = ?',
    [invoiceId],
  )
  if (invoiceData) {
    invoice.value = invoiceData

    if (invoiceData.bank_account_id) {
      const bankData = await window.electronAPI.dbGet<BankAccount>(
        'SELECT * FROM bank_accounts WHERE id = ?',
        [invoiceData.bank_account_id]
      )
      if (bankData) bankAccount.value = bankData
    }

    const clientData = await window.electronAPI.dbGet<Client>(
      'SELECT * FROM clients WHERE id = ?',
      [invoiceData.client_id],
    )
    if (clientData) client.value = clientData

    items.value = await window.electronAPI.dbQuery<InvoiceItem>(
      'SELECT * FROM invoice_items WHERE invoice_id = ?',
      [invoiceId],
    )

    payments.value = await window.electronAPI.dbQuery<InvoicePayment>(
      'SELECT * FROM invoice_payments WHERE invoice_id = ? ORDER BY date ASC',
      [invoiceId],
    )
  }
}

const print = () => {
  window.print()
}

onMounted(loadData)
</script>

<template>
  <div class="print-container min-h-screen bg-white">
    <!-- Controls (hidden when printing) -->
    <div class="print-controls p-4 bg-muted/50 border-b flex justify-between items-center no-print">
      <Button variant="outline" @click="router.back()" class="gap-2">
        <ArrowLeft class="h-4 w-4" />
        Volver
      </Button>
      <Button @click="print" class="gap-2">
        <Printer class="h-4 w-4" />
        Imprimir
      </Button>
    </div>

    <!-- Invoice Content -->
    <div class="max-w-[800px] mx-auto p-12 bg-white text-black print-area design-orlando">
      <div class="blue-line"></div>
      
      <div class="header-section">
        <h1 class="header-title">Cuenta de Cobro</h1>
        <div class="header-number">
          <p class="label">Número:</p>
          <p class="number">{{ invoice.number?.toString().padStart(5, '0') }}</p>
        </div>
      </div>
      
      <div class="info-grid">
        <div class="info-row">
          <div class="info-cell info-label">Nombre</div>
          <div class="info-cell info-value">{{ client.name?.toUpperCase() }}</div>
          <div class="info-cell info-label">Fecha :</div>
          <div class="info-cell info-value">{{ invoice.date }}</div>
        </div>
        <div class="info-row">
          <div class="info-cell info-label">Dirección</div>
          <div class="info-cell info-value">{{ client.address || 'N/A' }}</div>
          <div class="info-cell info-label">Teléfono</div>
          <div class="info-cell info-value">{{ client.phone || 'N/A' }}</div>
        </div>
        <div class="info-row">
          <div class="info-cell info-label">Ciudad</div>
          <div class="info-cell info-value">{{ client.city || 'N/A' }}</div>
          <div class="info-cell"></div>
          <div class="info-cell"></div>
        </div>
        <div class="info-row">
          <div class="info-cell info-label">ID / NIT:</div>
          <div class="info-cell info-value">{{ client.document_id }}</div>
          <div class="info-cell"></div>
          <div class="info-cell"></div>
        </div>
      </div>
      
      <div class="debe-section">
        <div class="debe-line">
          <div class="debe-item"><strong>Debe a:</strong> {{ profile.name }}</div>
          <div class="debe-item"><strong>ID:</strong> {{ profile.document_id }}</div>
          <div class="debe-item"><strong>de</strong> {{ profile.address?.split(',').pop()?.trim() || 'Colombia' }}</div>
        </div>
      </div>
      
      <table class="tabla-items">
        <thead>
          <tr>
            <th class="cantidad">Cantidad</th>
            <th class="descripcion">Descripción</th>
            <th class="valor-unitario">Valor unitario</th>
            <th class="total">TOTAL</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in items" :key="item.id">
            <td class="cantidad">{{ item.quantity }}</td>
            <td class="descripcion">{{ item.description }}</td>
            <td class="valor-unitario currency">{{ (item.price || 0).toLocaleString('es-CO', { minimumFractionDigits: 2 }) }}</td>
            <td class="total currency">{{ (item.quantity * item.price).toLocaleString('es-CO', { minimumFractionDigits: 2 }) }}</td>
          </tr>
          
          <!-- Filas vacías para completar el formato (total de 10 filas) -->
          <tr v-for="i in Math.max(0, 10 - items.length)" :key="'empty-' + i" class="empty-row">
            <td>-</td>
            <td>-</td>
            <td>-</td>
            <td>-</td>
          </tr>
        </tbody>
      </table>
      
      <div class="subtotal-section">
        <div class="subtotal-line">Subtotal: {{ (invoice.total || 0).toLocaleString('es-CO', { minimumFractionDigits: 2 }) }}</div>
      </div>
      
      <div v-if="bankAccount" class="pago-section">
        <div class="pago-title">Pago</div>
        <div>{{ bankAccount.bank }} {{ bankAccount.account_type }} {{ bankAccount.account_number }}</div>
      </div>
      <div v-else-if="profile.bank_info" class="pago-section">
        <div class="pago-title">Pago</div>
        <div>{{ profile.bank_info }}</div>
      </div>
      
      <div class="total-line">
        <strong>TOTAL: {{ (invoice.total || 0).toLocaleString('es-CO', { minimumFractionDigits: 2 }) }}</strong>
      </div>
      
      <div class="concepto-section">
        <div class="concepto-title">Concepto</div>
        <div class="concepto-text">
          Mis ingresos son rentas laborales y solicito aplicar la retención fuente art. 383 ET, manifiesto 
          que no aplicare costos y deducciones en mi declaración de renta.
        </div>
      </div>
      
      <div class="firma-section">
        <div class="firma-line"></div>
        <div class="firma-text">{{ profile.name }}</div>
        <div class="firma-text">ID: {{ profile.document_id }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.design-orlando {
    font-family: Arial, sans-serif;
    font-size: 12px;
    line-height: 1.4;
    color: #000;
}

.header-section {
    position: relative;
    margin-bottom: 30px;
}

.header-title {
    text-align: center;
    font-size: 18px;
    font-weight: bold;
    text-transform: uppercase;
    letter-spacing: 2px;
    margin: 0;
    padding: 10px 0;
}

.header-number {
    position: absolute;
    top: 0;
    right: 0;
    border: 2px solid #000;
    padding: 5px 10px;
    font-size: 12px;
    background: white;
}

.header-number .label {
    margin: 0;
    font-weight: normal;
}

.header-number .number {
    margin: 0;
    font-weight: bold;
}

.info-grid {
    display: table;
    width: 100%;
    margin: 20px 0;
    border-collapse: collapse;
}

.info-row {
    display: table-row;
}

.info-cell {
    display: table-cell;
    padding: 3px 10px 3px 0;
    vertical-align: top;
    font-size: 11px;
}

.info-label {
    font-weight: bold;
    width: 80px;
}

.info-value {
    width: 200px;
}

.debe-section {
    margin: 20px 0;
}

.debe-line {
    font-size: 12px;
    margin: 5px 0;
    width: 100%;
    position: relative;
}

.debe-item {
    display: inline-block;
    width: 32%;
}

.debe-item:first-child {
    text-align: left;
}

.debe-item:nth-child(2) {
    text-align: center;
}

.debe-item:last-child {
    text-align: right;
}

.tabla-items {
    width: 100%;
    border-collapse: collapse;
    margin: 5px 0;
    border: 1px solid #000;
}

.tabla-items th,
.tabla-items td {
    border-left: 1px solid #000;
    border-right: 1px solid #000;
    padding: 4px;
    text-align: center;
    font-size: 10px;
}

.tabla-items th {
    background: #f0f0f0;
    font-weight: bold;
    border-top: 1px solid #000;
    border-bottom: 1px solid #000;
}

.tabla-items tbody tr:last-child td {
    border-bottom: 1px solid #000;
}

.tabla-items .descripcion {
    text-align: left;
    width: 50%;
}

.tabla-items .cantidad {
    width: 15%;
}

.tabla-items .valor-unitario,
.tabla-items .total {
    width: 17.5%;
    text-align: right;
}

.empty-row td {
    height: 20px;
    color: #ccc;
}

.subtotal-section {
    margin: 20px 0;
    text-align: right;
}

.subtotal-line {
    margin: 5px 0;
    font-size: 12px;
}

.total-line {
    font-weight: bold;
    font-size: 14px;
    margin: 10px 0;
}

.pago-section {
    margin: 20px 0;
    font-size: 12px;
}

.pago-title {
    font-weight: bold;
    margin-bottom: 10px;
}

.concepto-section {
    margin: 30px 0;
    font-size: 11px;
}

.concepto-title {
    font-weight: bold;
    margin-bottom: 10px;
}

.concepto-text {
    text-align: justify;
    line-height: 1.5;
}

.firma-section {
    margin-top: 40px;
    text-align: center;
}

.firma-line {
    border-bottom: 1px solid #000;
    width: 300px;
    margin: 0 auto 10px auto;
    height: 5px;
}

.firma-text {
    font-size: 11px;
}

.currency {
    font-family: 'Courier New', monospace;
}

.blue-line {
    height: 3px;
    background-color: #4472C4;
    margin: 10px 0;
}

@media print {
  .no-print {
    display: none !important;
  }
  .print-container {
    background: transparent !important;
    min-height: auto !important;
  }
  .print-area {
    padding: 0 !important;
    max-width: 100% !important;
    margin: 0 !important;
  }
  @page {
    margin: 1.5cm;
  }
}
</style>
