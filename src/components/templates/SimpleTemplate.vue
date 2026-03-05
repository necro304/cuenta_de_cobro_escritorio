<script setup lang="ts">
import type { Profile, Client, Invoice, InvoiceItem, BankAccount } from '@/types'
import { numeroALetras } from '@/lib/numeroALetras'

defineProps<{
  profile: Partial<Profile>
  client: Partial<Client>
  invoice: Partial<Invoice>
  items: InvoiceItem[]
  bankAccount: BankAccount | null
}>()

const formatDate = (dateStr: string | undefined) => {
  if (!dateStr) return ''
  const parts = dateStr.split('-')
  if (parts.length === 3) {
    return `${parts[2]}/${parts[1]}/${parts[0]}`
  }
  return dateStr
}

const getDocumentType = (docId: string | undefined) => {
  // Basic heuristic or default to NIT/CC
  if (!docId) return 'CC'
  if (docId.includes('-')) return 'NIT'
  return 'CC'
}
</script>

<template>
  <div class="simple-template print-area bg-white text-black max-w-[800px] mx-auto p-10">
    <div class="fecha">Fecha: {{ formatDate(invoice.date) }}</div>

    <div class="empresa">
      <h1>{{ client.name }}</h1>
      <div class="nit">{{ getDocumentType(client.document_id) }}. {{ client.document_id }}</div>
    </div>

    <div class="debe-a">
      <h2>DEBE A:</h2>
      <div class="cliente-info">
        {{ profile.name?.toUpperCase() }}<br />
        {{ getDocumentType(profile.document_id).toUpperCase() }}: {{ profile.document_id }}<br />
      </div>
    </div>

    <div class="suma">
      <h2>LA SUMA DE:</h2>
      <div class="monto">
        {{ numeroALetras(invoice.total || 0) }} PESOS (COP
        {{ (invoice.total || 0).toLocaleString('es-CO', { minimumFractionDigits: 0 }) }})
      </div>
    </div>

    <div class="concepto">
      <h3>Concepto:</h3>
      <div class="concepto-detalle">
        <ul v-if="items.length > 0">
          <li v-for="item in items" :key="item.id">
            {{ item.description || 'Sin título' }}
          </li>
        </ul>
        <ul v-else>
          <li>Sin descripción</li>
        </ul>
      </div>
    </div>

    <div class="espaciado"></div>
    <div class="espaciado"></div>

    <div v-if="bankAccount" class="info-bancaria">
      <p><strong>Banco:</strong> {{ bankAccount.bank || 'Bancolombia' }}</p>
      <p><strong>Tipo de cuenta:</strong> {{ bankAccount.account_type }}</p>
      <p><strong>Número:</strong> {{ bankAccount.account_number }}</p>
    </div>
    <div v-else-if="profile.bank_info" class="info-bancaria">
      <p><strong>Información Bancaria:</strong> {{ profile.bank_info }}</p>
    </div>

    <div class="firma-section">
      <p>Cordialmente,</p>
      <!-- Si se agrega firma en base64 en profile en el futuro, se puede poner aquí -->
      <br /><br /><br />
      <p>{{ profile.name }}</p>
      <p>
        <strong>{{ getDocumentType(profile.document_id) }}:</strong> {{ profile.document_id }}
      </p>
      <p v-if="profile.phone"><strong>Teléfono:</strong> {{ profile.phone }}</p>
    </div>

    <div class="declaracion">
      "Declaro que no llevaré costos y deducciones asociados a los ingresos por rentas de<br />
      trabajo que no provienen de una relación laboral o legal y reglamentaria, y solicito que no<br />
      se me aplique la retención en la fuente conforme a la tabla del artículo 383 del<br />
      Estatuto Tributario."
    </div>
  </div>
</template>

<style scoped>
.simple-template {
  font-family: 'DejaVu Sans', Arial, sans-serif;
  font-size: 12px;
  line-height: 1.6;
  color: #000;
  margin: 0;
  padding: 40px;
  background: white;
}

.fecha {
  text-align: left;
  margin-bottom: 40px;
  font-weight: bold;
}

.empresa {
  text-align: center;
  margin-bottom: 40px;
}

.empresa h1 {
  font-size: 16px;
  font-weight: bold;
  margin: 0;
  text-transform: uppercase;
}

.empresa .nit {
  font-size: 12px;
  margin: 5px 0 0 0;
}

.debe-a {
  margin-bottom: 5px;
  text-align: center;
}

.debe-a h2 {
  font-size: 14px;
  font-weight: bold;
  margin: 0 0 1px 0;
  text-transform: uppercase;
}

.cliente-info {
  margin-left: 0;
  line-height: 1.1;
}

.suma {
  margin: 20px 0;
  text-align: center;
}

.suma h2 {
  font-size: 14px;
  font-weight: bold;
  margin: 0 0 0 0;
  text-transform: uppercase;
}

.monto {
  font-size: 14px;
  font-weight: bold;
  text-transform: uppercase;
}

.concepto {
  margin: 40px 0;
}

.concepto h3 {
  font-size: 12px;
  font-weight: bold;
  margin: 0 0 10px 0;
}

.concepto-detalle {
  margin-left: 20px;
}

.concepto-detalle ul {
  margin: 0;
  padding-left: 20px;
}

.concepto-detalle li {
  margin-bottom: 5px;
}

.info-bancaria {
  margin: 60px 0 40px 0;
}

.info-bancaria p {
  margin: 3px 0;
  font-size: 12px;
}

.declaracion {
  margin-top: 60px;
  text-align: center;
  font-size: 10px;
  line-height: 1.4;
  font-style: italic;
}

.firma-section {
  margin-top: 20px;
}

.firma-section p {
  margin: 2px 0;
  font-size: 11px;
  line-height: 1.3;
}

.espaciado {
  margin: 20px 0;
}

.signature-image {
  max-width: 200px;
  max-height: 80px;
  margin: 10px 0;
}
</style>
