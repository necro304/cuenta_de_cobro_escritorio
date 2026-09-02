<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { formatCurrency, formatDate } from '@/lib/format'
import type { Client, Invoice, InvoicePayment, Profile } from '@/types'

interface MonthlyAmount {
  month: string
  total: number
}

interface ClientPayment extends InvoicePayment {
  invoice_number: number
}

const route = useRoute()
const clientId = Number(Array.isArray(route.params.id) ? route.params.id[0] : route.params.id)
const isPdfMode = route.query.pdf === '1'

const client = ref<Client | null>(null)
const profile = ref<Profile | null>(null)
const invoices = ref<Invoice[]>([])
const monthlyInvoices = ref<MonthlyAmount[]>([])
const monthlyPayments = ref<MonthlyAmount[]>([])
const payments = ref<ClientPayment[]>([])
const isLoading = ref(true)
const loadError = ref(false)

const reportDate = new Intl.DateTimeFormat('es-CO', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
}).format(new Date())

const reportReference = `INF-${String(clientId).padStart(4, '0')}-${new Date()
  .toISOString()
  .slice(0, 10)
  .replace(/-/g, '')}`

const getInitials = (name: string) =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase())
    .join('')

const financialSummary = computed(() => {
  const summary = invoices.value.reduce(
    (result, invoice) => {
      const paid = Math.min(Math.max(invoice.paid_amount ?? 0, 0), invoice.total)
      result.issued += invoice.total
      result.collected += paid
      if (invoice.status !== 'paid') result.pendingCount += 1
      return result
    },
    { issued: 0, collected: 0, pendingCount: 0 },
  )

  return {
    ...summary,
    balance: Math.max(summary.issued - summary.collected, 0),
    average: invoices.value.length > 0 ? summary.issued / invoices.value.length : 0,
    collectionRate: summary.issued > 0 ? (summary.collected / summary.issued) * 100 : 0,
  }
})

const collectionLabel = computed(() => {
  if (invoices.value.length === 0) return 'Sin actividad registrada'
  if (financialSummary.value.collectionRate >= 99.99) return 'Cartera al día'
  if (financialSummary.value.collectionRate >= 70) return 'Recaudo saludable'
  if (financialSummary.value.collectionRate > 0) return 'Recaudo en seguimiento'
  return 'Cartera pendiente'
})

const periodLabel = computed(() => {
  if (invoices.value.length === 0) return 'Sin movimientos registrados'
  const orderedDates = invoices.value.map((invoice) => invoice.date).sort()
  return `${formatDate(orderedDates[0])} — ${formatDate(orderedDates[orderedDates.length - 1])}`
})

const executiveSummary = computed(() => {
  if (invoices.value.length === 0) {
    return 'La relación comercial todavía no registra cuentas de cobro ni movimientos de recaudo.'
  }

  const documentWord = invoices.value.length === 1 ? 'documento' : 'documentos'
  const pendingSummary =
    financialSummary.value.pendingCount === 0
      ? 'No hay saldos pendientes.'
      : `${financialSummary.value.pendingCount} ${financialSummary.value.pendingCount === 1 ? 'permanece' : 'permanecen'} con saldo pendiente.`
  return `Durante el periodo se emitieron ${invoices.value.length} ${documentWord} por $${formatCurrency(financialSummary.value.issued)}. Se ha recaudado el ${financialSummary.value.collectionRate.toFixed(1)}%. ${pendingSummary}`
})

const statusMeta: Record<Invoice['status'], { label: string; color: string; softColor: string }> = {
  paid: { label: 'Pagadas', color: '#2d8b66', softColor: '#dcece2' },
  partially_paid: { label: 'Con abonos', color: '#487a8e', softColor: '#e1ebef' },
  draft: { label: 'Pendientes', color: '#c28a35', softColor: '#f4ead9' },
}

const statusRows = computed(() =>
  (['paid', 'partially_paid', 'draft'] as const).map((status) => {
    const matchingInvoices = invoices.value.filter((invoice) => invoice.status === status)
    return {
      status,
      count: matchingInvoices.length,
      total: matchingInvoices.reduce((total, invoice) => total + invoice.total, 0),
      percentage:
        invoices.value.length > 0 ? (matchingInvoices.length / invoices.value.length) * 100 : 0,
      ...statusMeta[status],
    }
  }),
)

const lastSixMonths = computed(() => {
  const now = new Date()
  return Array.from({ length: 6 }, (_, index) => {
    const date = new Date(now.getFullYear(), now.getMonth() - 5 + index, 1)
    return {
      month: `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`,
      label: new Intl.DateTimeFormat('es-CO', { month: 'short' })
        .format(date)
        .replace('.', '')
        .toUpperCase(),
    }
  })
})

const monthlyRows = computed(() => {
  const issuedByMonth = new Map(monthlyInvoices.value.map((row) => [row.month, row.total]))
  const collectedByMonth = new Map(monthlyPayments.value.map((row) => [row.month, row.total]))
  return lastSixMonths.value.map((month) => ({
    ...month,
    issued: issuedByMonth.get(month.month) ?? 0,
    collected: collectedByMonth.get(month.month) ?? 0,
  }))
})

const maxMonthlyAmount = computed(() =>
  Math.max(...monthlyRows.value.flatMap((row) => [row.issued, row.collected]), 0),
)

const barHeight = (value: number) => {
  if (value <= 0 || maxMonthlyAmount.value <= 0) return '0%'
  return `${Math.max((value / maxMonthlyAmount.value) * 100, 5)}%`
}

const compactCurrency = (value: number) =>
  `$${Intl.NumberFormat('es-CO', { notation: 'compact', maximumFractionDigits: 1 }).format(value)}`

const statusLabel = (status: Invoice['status']) =>
  statusMeta[status].label.replace('Pagadas', 'Pagada').replace('Pendientes', 'Pendiente')

const signalReady = async () => {
  if (!isPdfMode) return
  await nextTick()
  if (document.fonts) await document.fonts.ready
  setTimeout(() => window.electronAPI.notifyPrintReady(), 100)
}

const loadReport = async () => {
  isLoading.value = true
  loadError.value = false

  try {
    const [
      clientRecord,
      profileRecord,
      clientInvoices,
      issuedByMonth,
      collectedByMonth,
      clientPayments,
    ] = await Promise.all([
      window.electronAPI.dbGet<Client>('SELECT * FROM clients WHERE id = ?', [clientId]),
      window.electronAPI.dbGet<Profile>('SELECT * FROM profile WHERE id = 1'),
      window.electronAPI.dbQuery<Invoice>(
        `SELECT i.*,
            COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0) as paid_amount,
            MAX(i.total - COALESCE((SELECT SUM(amount) FROM invoice_payments WHERE invoice_id = i.id), 0), 0) as balance
          FROM invoices i
          WHERE i.client_id = ?
          ORDER BY i.date DESC, i.number DESC`,
        [clientId],
      ),
      window.electronAPI.dbQuery<MonthlyAmount>(
        `SELECT strftime('%Y-%m', date) as month, COALESCE(SUM(total), 0) as total
          FROM invoices
          WHERE client_id = ?
            AND date >= date('now', 'localtime', 'start of month', '-5 months')
          GROUP BY month
          ORDER BY month`,
        [clientId],
      ),
      window.electronAPI.dbQuery<MonthlyAmount>(
        `SELECT strftime('%Y-%m', p.date) as month, COALESCE(SUM(p.amount), 0) as total
          FROM invoice_payments p
          JOIN invoices i ON i.id = p.invoice_id
          WHERE i.client_id = ?
            AND p.date >= date('now', 'localtime', 'start of month', '-5 months')
          GROUP BY month
          ORDER BY month`,
        [clientId],
      ),
      window.electronAPI.dbQuery<ClientPayment>(
        `SELECT p.*, i.number as invoice_number
          FROM invoice_payments p
          JOIN invoices i ON i.id = p.invoice_id
          WHERE i.client_id = ?
          ORDER BY p.date DESC, p.id DESC
          LIMIT 8`,
        [clientId],
      ),
    ])

    if (!clientRecord) {
      loadError.value = true
      return
    }

    client.value = clientRecord
    profile.value = profileRecord ?? null
    invoices.value = clientInvoices
    monthlyInvoices.value = issuedByMonth
    monthlyPayments.value = collectedByMonth
    payments.value = clientPayments
  } catch {
    loadError.value = true
  } finally {
    isLoading.value = false
    await signalReady()
  }
}

onMounted(loadReport)
</script>

<template>
  <main class="report-stage">
    <div v-if="isLoading" class="report-state">Preparando informe…</div>
    <div v-else-if="loadError || !client" class="report-state report-error">
      No fue posible generar el informe del cliente.
    </div>

    <article v-else class="report-document">
      <section class="report-page summary-page">
        <header class="report-hero">
          <div class="hero-orbit" aria-hidden="true"></div>
          <div class="hero-topline">
            <div class="report-brand">
              <span class="brand-mark">C</span>
              <div>
                <strong>CuentaCobro</strong>
                <span>Gestión local</span>
              </div>
            </div>
            <div class="report-meta">
              <span>{{ reportReference }}</span>
              <strong>{{ reportDate }}</strong>
            </div>
          </div>

          <div class="hero-heading">
            <p>Informe de relación comercial</p>
            <h1>Estado financiero<br />del cliente</h1>
          </div>

          <div class="client-identity">
            <div class="client-initials">{{ getInitials(client.name) }}</div>
            <div class="client-primary">
              <span>Preparado para</span>
              <h2>{{ client.name }}</h2>
            </div>
            <dl class="client-facts">
              <div>
                <dt>Documento</dt>
                <dd>{{ client.document_id || 'Sin registrar' }}</dd>
              </div>
              <div>
                <dt>Ciudad</dt>
                <dd>{{ client.city || 'Sin registrar' }}</dd>
              </div>
            </dl>
          </div>
        </header>

        <div class="report-content">
          <section class="balance-section">
            <div>
              <p class="eyebrow">CARTERA ACTUAL</p>
              <p class="balance-value">${{ formatCurrency(financialSummary.balance) }}</p>
              <p class="balance-caption">
                {{ financialSummary.pendingCount }}
                {{
                  financialSummary.pendingCount === 1 ? 'cuenta pendiente' : 'cuentas pendientes'
                }}
                · Periodo {{ periodLabel }}
              </p>
            </div>
            <div class="collection-score">
              <div
                class="score-ring"
                :style="{ '--score': `${financialSummary.collectionRate * 3.6}deg` }"
              >
                <div>
                  <strong>{{ financialSummary.collectionRate.toFixed(0) }}%</strong>
                  <span>recaudado</span>
                </div>
              </div>
              <p>{{ collectionLabel }}</p>
            </div>
          </section>

          <section class="metric-grid">
            <article>
              <span>Total emitido</span>
              <strong>${{ formatCurrency(financialSummary.issued) }}</strong>
              <small>{{ invoices.length }} documentos históricos</small>
            </article>
            <article>
              <span>Total recaudado</span>
              <strong>${{ formatCurrency(financialSummary.collected) }}</strong>
              <small>Pagos aplicados a cuentas</small>
            </article>
            <article>
              <span>Valor promedio</span>
              <strong>${{ formatCurrency(financialSummary.average) }}</strong>
              <small>Por documento emitido</small>
            </article>
          </section>

          <div class="analysis-grid">
            <section class="chart-panel">
              <div class="section-heading">
                <div>
                  <p class="eyebrow">ÚLTIMOS 6 MESES</p>
                  <h3>Emitido frente a recaudado</h3>
                </div>
                <div class="chart-legend">
                  <span><i class="issued-dot"></i> Emitido</span>
                  <span><i class="collected-dot"></i> Recaudado</span>
                </div>
              </div>

              <div class="chart-area">
                <div class="chart-grid" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
                <div v-for="month in monthlyRows" :key="month.month" class="chart-column">
                  <div class="bars">
                    <span
                      class="chart-bar issued-bar"
                      :style="{ height: barHeight(month.issued) }"
                      :title="`Emitido ${compactCurrency(month.issued)}`"
                    ></span>
                    <span
                      class="chart-bar collected-bar"
                      :style="{ height: barHeight(month.collected) }"
                      :title="`Recaudado ${compactCurrency(month.collected)}`"
                    ></span>
                  </div>
                  <span class="month-label">{{ month.label }}</span>
                </div>
              </div>
            </section>

            <section class="status-panel">
              <div class="section-heading">
                <div>
                  <p class="eyebrow">PORTAFOLIO</p>
                  <h3>Estado de cuentas</h3>
                </div>
              </div>
              <div class="status-list">
                <div v-for="row in statusRows" :key="row.status" class="status-row">
                  <div class="status-copy">
                    <i :style="{ background: row.color }"></i>
                    <div>
                      <strong>{{ row.label }}</strong>
                      <span>${{ formatCurrency(row.total) }}</span>
                    </div>
                    <b>{{ row.count }}</b>
                  </div>
                  <div class="status-track">
                    <span
                      :style="{
                        width: `${row.percentage}%`,
                        background: row.color,
                      }"
                    ></span>
                  </div>
                </div>
              </div>
            </section>
          </div>

          <section class="executive-note">
            <span>Lectura ejecutiva</span>
            <p>{{ executiveSummary }}</p>
          </section>
        </div>

        <footer class="report-footer">
          <span>Generado por {{ profile?.name || 'CuentaCobro' }}</span>
          <span>Información consolidada al {{ reportDate }}</span>
          <strong>01</strong>
        </footer>
      </section>

      <section v-if="invoices.length > 0" class="report-page movements-page">
        <header class="page-header-compact">
          <div class="report-brand dark-brand">
            <span class="brand-mark">C</span>
            <div>
              <strong>CuentaCobro</strong>
              <span>Informe financiero</span>
            </div>
          </div>
          <div class="compact-client">
            <span>{{ client.document_id || 'Sin documento' }}</span>
            <strong>{{ client.name }}</strong>
          </div>
        </header>

        <div class="movement-content">
          <div class="movement-intro">
            <div>
              <p class="eyebrow">DETALLE DE MOVIMIENTOS</p>
              <h2>Cuentas y recaudos</h2>
              <p>Relación de los movimientos más recientes incluidos en el resumen.</p>
            </div>
            <div class="period-card">
              <span>Periodo analizado</span>
              <strong>{{ periodLabel }}</strong>
            </div>
          </div>

          <section class="table-section">
            <div class="table-heading">
              <div>
                <h3>Últimas cuentas emitidas</h3>
                <p v-if="invoices.length > 10">Mostrando 10 de {{ invoices.length }} documentos</p>
                <p v-else>{{ invoices.length }} documentos en el historial</p>
              </div>
              <strong>${{ formatCurrency(financialSummary.issued) }}</strong>
            </div>

            <table class="report-table">
              <thead>
                <tr>
                  <th>Cuenta</th>
                  <th>Fecha</th>
                  <th>Estado</th>
                  <th>Total</th>
                  <th>Recaudado</th>
                  <th>Saldo</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="invoice in invoices.slice(0, 10)" :key="invoice.id">
                  <td class="invoice-number">#{{ invoice.number }}</td>
                  <td>{{ formatDate(invoice.date) }}</td>
                  <td>
                    <span class="status-badge" :class="`status-${invoice.status}`">
                      {{ statusLabel(invoice.status) }}
                    </span>
                  </td>
                  <td>${{ formatCurrency(invoice.total) }}</td>
                  <td>${{ formatCurrency(invoice.paid_amount) }}</td>
                  <td class="balance-cell">${{ formatCurrency(invoice.balance) }}</td>
                </tr>
              </tbody>
            </table>
          </section>

          <section class="payments-section">
            <div class="table-heading">
              <div>
                <h3>Abonos recientes</h3>
                <p>Pagos aplicados a las cuentas del cliente</p>
              </div>
              <strong>${{ formatCurrency(financialSummary.collected) }}</strong>
            </div>

            <div v-if="payments.length > 0" class="payment-grid">
              <article v-for="payment in payments" :key="payment.id">
                <div class="payment-icon">$</div>
                <div>
                  <span>Cuenta #{{ payment.invoice_number }} · {{ formatDate(payment.date) }}</span>
                  <p>{{ payment.notes || 'Abono registrado' }}</p>
                </div>
                <strong>+${{ formatCurrency(payment.amount) }}</strong>
              </article>
            </div>
            <div v-else class="payments-empty">No se registran abonos para este cliente.</div>
          </section>

          <section class="contact-strip">
            <div>
              <span>Contacto del cliente</span>
              <strong>{{ client.email || 'Correo sin registrar' }}</strong>
            </div>
            <div>
              <span>Teléfono</span>
              <strong>{{ client.phone || 'Sin registrar' }}</strong>
            </div>
            <div>
              <span>Ubicación</span>
              <strong>{{ client.city || 'Sin ciudad' }}</strong>
            </div>
          </section>
        </div>

        <footer class="report-footer dark-footer">
          <span>{{ reportReference }}</span>
          <span>Documento informativo · Valores expresados en COP</span>
          <strong>02</strong>
        </footer>
      </section>
    </article>
  </main>
</template>

<style scoped>
:global(html:has(.report-stage)),
:global(body:has(.report-stage)) {
  overflow: auto;
  background: #e9ece8;
}

:global(body:has(.report-stage)::before) {
  display: none;
}

.report-stage {
  min-height: 100dvh;
  padding: 24px;
  background: #e9ece8;
  color: #14261f;
  font-family: 'Outfit Variable', 'Avenir Next', sans-serif;
}

.report-state {
  display: flex;
  width: 210mm;
  min-height: 297mm;
  margin: 0 auto;
  align-items: center;
  justify-content: center;
  background: #fbfaf5;
  color: #627068;
  font-size: 14px;
}

.report-error {
  color: #a63f36;
}

.report-document {
  width: 210mm;
  margin: 0 auto;
  box-shadow: 0 24px 80px rgb(20 38 31 / 16%);
}

.report-page {
  position: relative;
  box-sizing: border-box;
  width: 210mm;
  min-height: 297mm;
  overflow: hidden;
  background: #fbfaf5;
  break-after: page;
}

.report-page:last-child {
  break-after: auto;
}

.report-hero {
  position: relative;
  height: 81mm;
  overflow: hidden;
  padding: 13mm 15mm 0;
  background: #245f46;
  color: #f8fbf8;
}

.hero-orbit {
  position: absolute;
  width: 120mm;
  height: 120mm;
  right: -46mm;
  bottom: -72mm;
  border: 0.3mm solid rgb(255 255 255 / 15%);
  border-radius: 50%;
  box-shadow:
    0 0 0 14mm rgb(255 255 255 / 3.5%),
    0 0 0 30mm rgb(255 255 255 / 2.5%);
}

.hero-topline,
.page-header-compact {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.report-brand {
  display: flex;
  align-items: center;
  gap: 3mm;
}

.brand-mark {
  display: flex;
  width: 9mm;
  height: 9mm;
  align-items: center;
  justify-content: center;
  border: 0.3mm solid rgb(255 255 255 / 25%);
  border-radius: 2.5mm;
  background: #d7f2e3;
  color: #245f46;
  font-size: 14px;
  font-weight: 750;
}

.report-brand div {
  display: grid;
}

.report-brand strong {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.report-brand span:not(.brand-mark) {
  color: rgb(255 255 255 / 57%);
  font-size: 7px;
}

.report-meta {
  display: grid;
  gap: 1mm;
  text-align: right;
}

.report-meta span {
  color: rgb(255 255 255 / 55%);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 6.5px;
  letter-spacing: 0.08em;
}

.report-meta strong {
  font-size: 8px;
  font-weight: 500;
}

.hero-heading {
  position: relative;
  z-index: 1;
  margin-top: 10mm;
}

.hero-heading p,
.eyebrow {
  color: #4f9b79;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 7px;
  font-weight: 700;
  letter-spacing: 0.16em;
}

.hero-heading p {
  color: #a8d9c2;
}

.hero-heading h1 {
  margin: 2mm 0 0;
  color: #fff;
  font-size: 25px;
  font-weight: 620;
  letter-spacing: -0.045em;
  line-height: 0.96;
}

.client-identity {
  position: absolute;
  z-index: 2;
  right: 15mm;
  bottom: 0;
  left: 15mm;
  display: grid;
  min-height: 21mm;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 4mm;
  padding: 4mm 5mm;
  border: 0.3mm solid rgb(255 255 255 / 14%);
  border-bottom: 0;
  border-radius: 4mm 4mm 0 0;
  background: rgb(255 255 255 / 9%);
}

.client-initials {
  display: flex;
  width: 11mm;
  height: 11mm;
  align-items: center;
  justify-content: center;
  border-radius: 3mm;
  background: #d7f2e3;
  color: #245f46;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 9px;
  font-weight: 700;
}

.client-primary {
  min-width: 0;
}

.client-primary span,
.client-facts dt {
  display: block;
  color: rgb(255 255 255 / 54%);
  font-size: 6.5px;
}

.client-primary h2 {
  overflow: hidden;
  margin: 0.7mm 0 0;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.client-facts {
  display: flex;
  gap: 8mm;
  margin: 0;
}

.client-facts div {
  min-width: 23mm;
}

.client-facts dd {
  margin: 0.7mm 0 0;
  font-size: 8px;
  font-weight: 550;
}

.report-content {
  padding: 10mm 15mm 13mm;
}

.balance-section {
  display: flex;
  min-height: 34mm;
  align-items: center;
  justify-content: space-between;
  border-bottom: 0.3mm solid #d9ddd5;
}

.balance-value {
  margin: 2mm 0 0;
  color: #14261f;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 26px;
  font-weight: 650;
  letter-spacing: -0.06em;
  line-height: 1;
}

.balance-caption {
  margin: 2mm 0 0;
  color: #6e7972;
  font-size: 7px;
}

.collection-score {
  display: flex;
  align-items: center;
  gap: 3mm;
}

.score-ring {
  display: flex;
  width: 21mm;
  height: 21mm;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: conic-gradient(#3b8f6b var(--score), #e2e7e1 0deg);
}

.score-ring > div {
  display: flex;
  width: 16.5mm;
  height: 16.5mm;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: #fbfaf5;
}

.score-ring strong {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
  letter-spacing: -0.04em;
}

.score-ring span {
  margin-top: 0.4mm;
  color: #78827c;
  font-size: 5.5px;
}

.collection-score > p {
  width: 24mm;
  margin: 0;
  color: #496058;
  font-size: 8px;
  font-weight: 600;
  line-height: 1.25;
}

.metric-grid {
  display: grid;
  margin-top: 7mm;
  grid-template-columns: repeat(3, 1fr);
  border: 0.3mm solid #d9ddd5;
  border-radius: 3.5mm;
  background: #f6f5ef;
}

.metric-grid article {
  display: grid;
  min-height: 24mm;
  align-content: center;
  padding: 4mm 5mm;
  border-right: 0.3mm solid #d9ddd5;
}

.metric-grid article:last-child {
  border-right: 0;
}

.metric-grid span {
  color: #6e7972;
  font-size: 6.5px;
  font-weight: 550;
}

.metric-grid strong {
  margin-top: 2mm;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 12px;
  font-weight: 650;
  letter-spacing: -0.04em;
}

.metric-grid small {
  margin-top: 1mm;
  color: #89918c;
  font-size: 5.8px;
}

.analysis-grid {
  display: grid;
  margin-top: 7mm;
  grid-template-columns: 1.65fr 1fr;
  gap: 5mm;
}

.chart-panel,
.status-panel {
  min-height: 69mm;
  padding: 5mm;
  border: 0.3mm solid #d9ddd5;
  border-radius: 3.5mm;
  background: #fff;
}

.section-heading,
.table-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 4mm;
}

.section-heading h3,
.table-heading h3 {
  margin: 1.4mm 0 0;
  color: #14261f;
  font-size: 10px;
  font-weight: 650;
  letter-spacing: -0.025em;
}

.chart-legend {
  display: flex;
  gap: 3mm;
  color: #6e7972;
  font-size: 5.8px;
}

.chart-legend span {
  display: flex;
  align-items: center;
  gap: 1.2mm;
}

.chart-legend i {
  width: 2mm;
  height: 2mm;
  border-radius: 0.6mm;
}

.issued-dot,
.issued-bar {
  background: #245f46;
}

.collected-dot,
.collected-bar {
  background: #78cfa7;
}

.chart-area {
  position: relative;
  display: grid;
  height: 46mm;
  margin-top: 5mm;
  grid-template-columns: repeat(6, 1fr);
  gap: 2mm;
  border-bottom: 0.3mm solid #cfd5ce;
}

.chart-grid {
  position: absolute;
  inset: 0 0 5mm;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.chart-grid i {
  display: block;
  border-top: 0.3mm solid #e6e9e4;
}

.chart-column {
  position: relative;
  z-index: 1;
  display: grid;
  min-width: 0;
  grid-template-rows: 1fr 5mm;
}

.bars {
  display: flex;
  align-items: flex-end;
  justify-content: center;
  gap: 1mm;
}

.chart-bar {
  width: 4.2mm;
  max-height: 100%;
  border-radius: 1mm 1mm 0 0;
}

.month-label {
  align-self: end;
  color: #758079;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 5.8px;
  font-weight: 650;
  text-align: center;
}

.status-list {
  display: grid;
  margin-top: 6mm;
  gap: 5mm;
}

.status-copy {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 2mm;
}

.status-copy > i {
  width: 2mm;
  height: 2mm;
  border-radius: 50%;
}

.status-copy div {
  display: grid;
}

.status-copy strong,
.status-copy b {
  font-size: 7.5px;
  font-weight: 650;
}

.status-copy span {
  margin-top: 0.5mm;
  color: #7c857f;
  font-size: 5.8px;
}

.status-track {
  height: 1.5mm;
  margin: 2mm 0 0 4mm;
  overflow: hidden;
  border-radius: 1mm;
  background: #eceee9;
}

.status-track span {
  display: block;
  height: 100%;
  border-radius: inherit;
}

.executive-note {
  display: grid;
  margin-top: 6mm;
  grid-template-columns: 33mm 1fr;
  gap: 5mm;
  padding: 4mm 5mm;
  border-left: 1mm solid #4f9b79;
  background: #edf4ee;
}

.executive-note span {
  color: #245f46;
  font-size: 7px;
  font-weight: 650;
}

.executive-note p {
  margin: 0;
  color: #496058;
  font-size: 7px;
  line-height: 1.45;
}

.report-footer {
  position: absolute;
  right: 15mm;
  bottom: 7mm;
  left: 15mm;
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  align-items: center;
  padding-top: 3mm;
  border-top: 0.3mm solid #d9ddd5;
  color: #838b86;
  font-size: 5.8px;
}

.report-footer span:nth-child(2) {
  text-align: center;
}

.report-footer strong {
  color: #245f46;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 7px;
}

.movements-page {
  padding: 12mm 15mm 14mm;
}

.page-header-compact {
  padding-bottom: 6mm;
  border-bottom: 0.5mm solid #245f46;
}

.dark-brand .brand-mark {
  border-color: #c7d9ce;
}

.dark-brand strong {
  color: #14261f;
}

.dark-brand span:not(.brand-mark) {
  color: #7b867f;
}

.compact-client {
  display: grid;
  max-width: 90mm;
  text-align: right;
}

.compact-client span {
  color: #78827c;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 6px;
}

.compact-client strong {
  overflow: hidden;
  margin-top: 1mm;
  font-size: 9px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.movement-content {
  padding-top: 10mm;
}

.movement-intro {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 8mm;
}

.movement-intro h2 {
  margin: 1.8mm 0 0;
  font-size: 22px;
  font-weight: 620;
  letter-spacing: -0.045em;
  line-height: 1;
}

.movement-intro > div > p:last-child {
  margin: 2mm 0 0;
  color: #6e7972;
  font-size: 7px;
}

.period-card {
  display: grid;
  min-width: 52mm;
  padding: 3.5mm 4mm;
  border-radius: 2.5mm;
  background: #edf4ee;
}

.period-card span {
  color: #6a786f;
  font-size: 6px;
}

.period-card strong {
  margin-top: 1mm;
  color: #245f46;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 7px;
}

.table-section,
.payments-section {
  margin-top: 10mm;
}

.table-heading {
  align-items: flex-end;
  margin-bottom: 4mm;
}

.table-heading h3 {
  margin: 0;
  font-size: 11px;
}

.table-heading p {
  margin: 1mm 0 0;
  color: #7b857f;
  font-size: 6.5px;
}

.table-heading > strong {
  color: #245f46;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 10px;
  font-weight: 650;
}

.report-table {
  width: 100%;
  overflow: hidden;
  border: 0.3mm solid #d9ddd5;
  border-collapse: separate;
  border-spacing: 0;
  border-radius: 3mm;
  font-size: 6.7px;
}

.report-table th {
  height: 9mm;
  padding: 0 3mm;
  background: #edf0eb;
  color: #68756d;
  font-size: 5.8px;
  font-weight: 650;
  text-align: left;
}

.report-table th:nth-child(n + 4),
.report-table td:nth-child(n + 4) {
  text-align: right;
}

.report-table td {
  height: 9.5mm;
  padding: 0 3mm;
  border-top: 0.3mm solid #e2e5df;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  white-space: nowrap;
}

.invoice-number,
.balance-cell {
  color: #245f46;
  font-weight: 700;
}

.status-badge {
  display: inline-flex;
  padding: 1.2mm 2mm;
  border-radius: 1.5mm;
  font-family: 'Outfit Variable', 'Avenir Next', sans-serif;
  font-size: 5.8px;
  font-weight: 650;
}

.status-paid {
  background: #dcece2;
  color: #226647;
}

.status-partially_paid {
  background: #e1ebef;
  color: #376779;
}

.status-draft {
  background: #f4ead9;
  color: #946623;
}

.payment-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 2.5mm;
}

.payment-grid article {
  display: grid;
  min-width: 0;
  min-height: 14mm;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 2.5mm;
  padding: 2.5mm 3mm;
  border: 0.3mm solid #dfe3dd;
  border-radius: 2.5mm;
  background: #fff;
}

.payment-icon {
  display: flex;
  width: 7mm;
  height: 7mm;
  align-items: center;
  justify-content: center;
  border-radius: 2mm;
  background: #dcece2;
  color: #245f46;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 7px;
  font-weight: 700;
}

.payment-grid article div:nth-child(2) {
  min-width: 0;
}

.payment-grid span {
  display: block;
  color: #6e7972;
  font-size: 5.8px;
}

.payment-grid p {
  overflow: hidden;
  margin: 1mm 0 0;
  font-size: 7px;
  font-weight: 550;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.payment-grid article > strong {
  color: #2d8b66;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 7px;
  font-weight: 700;
  white-space: nowrap;
}

.payments-empty {
  padding: 8mm;
  border: 0.3mm dashed #cfd5ce;
  border-radius: 3mm;
  color: #78827c;
  font-size: 7px;
  text-align: center;
}

.contact-strip {
  display: grid;
  margin-top: 9mm;
  grid-template-columns: 1.4fr 1fr 1fr;
  padding: 5mm;
  border-radius: 3mm;
  background: #245f46;
  color: #fff;
}

.contact-strip div {
  display: grid;
  padding: 0 4mm;
  border-right: 0.3mm solid rgb(255 255 255 / 17%);
}

.contact-strip div:first-child {
  padding-left: 0;
}

.contact-strip div:last-child {
  padding-right: 0;
  border-right: 0;
}

.contact-strip span {
  color: rgb(255 255 255 / 58%);
  font-size: 6px;
}

.contact-strip strong {
  overflow: hidden;
  margin-top: 1mm;
  font-size: 7.5px;
  font-weight: 550;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dark-footer {
  right: 0;
  bottom: 7mm;
  left: 0;
}

@media print {
  :global(html),
  :global(body) {
    width: 210mm;
    margin: 0 !important;
    overflow: visible !important;
    background: #fbfaf5 !important;
  }

  .report-stage {
    min-height: auto;
    padding: 0;
    background: transparent;
  }

  .report-document {
    margin: 0;
    box-shadow: none;
  }

  .report-page {
    height: 297mm;
    min-height: 297mm;
  }

  @page {
    size: A4;
    margin: 0;
  }
}
</style>
