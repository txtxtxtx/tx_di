<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '../api/can'
import { pushLog } from '../store'
import { t } from '../i18n'
import type { AuditEntryInfo } from '../types'

const entries = ref<AuditEntryInfo[]>([])

async function refresh() {
  try {
    entries.value = await api.auditLog()
  } catch (e) {
    pushLog('审计读取失败: ' + String(e))
  }
}
async function doClear() {
  await api.auditClear()
  entries.value = []
  pushLog('审计已清空')
}
async function doExport(fmt: string) {
  const path = fmt === 'pdf' ? 'audit_report.pdf' : 'audit_report.html'
  try {
    await api.exportReport(path, fmt)
    pushLog(`已导出 ${fmt.toUpperCase()} 报表: ${path}`)
  } catch (e) {
    pushLog('导出失败: ' + String(e))
  }
}
function fmtTime(ts: number): string {
  const d = new Date(ts)
  return d.toLocaleTimeString()
}
onMounted(refresh)
</script>

<template>
  <div class="view">
    <h3>{{ t('title.audit') }}</h3>
    <section class="panel">
      <div class="row">
        <button @click="refresh">{{ t('common.refresh') }}</button>
        <button @click="doExport('html')">导出 HTML</button>
        <button @click="doExport('pdf')">导出 PDF</button>
        <button @click="doClear">{{ t('common.clear') }}</button>
        <span class="muted">共 {{ entries.length }} 条</span>
      </div>
    </section>

    <section class="panel grow">
      <div class="table-wrap" style="max-height: 420px">
        <table>
          <thead>
            <tr><th>时间</th><th>类型</th><th>详情</th><th>结果</th></tr>
          </thead>
          <tbody>
            <tr v-for="(e, i) in entries" :key="i">
              <td class="mono">{{ fmtTime(e.ts_ms) }}</td>
              <td>{{ e.kind }}</td>
              <td>{{ e.detail }}</td>
              <td :class="e.result === 'ok' ? 'ok' : 'fail'">{{ e.result }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
