<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/can'
import { pushLog } from '../store'
import type { CsvAnalysis } from '../types'

const recPath = ref('record.csv')
const recDuration = ref(3000)
const recResult = ref('')

const repPath = ref('record.csv')
const repSpeed = ref(1.0)
const repResult = ref('')

const anaPath = ref('record.csv')
const anaBitrate = ref(500000)
const analysis = ref<CsvAnalysis | null>(null)

async function doRecord() {
  recResult.value = '录制中...'
  try {
    const n = await api.recordCsv(recPath.value, recDuration.value)
    recResult.value = `已录制 ${n} 帧 → ${recPath.value}`
    pushLog(`录制 ${n} 帧`)
  } catch (e) {
    recResult.value = '失败: ' + String(e)
  }
}
async function doReplay() {
  repResult.value = '回放中...'
  try {
    const n = await api.replayCsv(repPath.value, repSpeed.value)
    repResult.value = `已回放 ${n} 帧 (${repSpeed.value}x)`
    pushLog(`回放 ${n} 帧`)
  } catch (e) {
    repResult.value = '失败: ' + String(e)
  }
}
async function doAnalyze() {
  try {
    analysis.value = await api.analyzeCsv(anaPath.value, anaBitrate.value)
    pushLog(`离线分析完成: ${analysis.value.total_frames} 帧`)
  } catch (e) {
    pushLog('分析失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <section class="panel">
      <h3>录制 (CSV)</h3>
      <div class="row">
        <label>输出路径</label><input v-model="recPath" style="width: 240px" />
        <label>时长(ms)</label><input v-model.number="recDuration" style="width: 90px" />
        <button @click="doRecord">开始录制</button>
      </div>
      <pre>{{ recResult || '待录制' }}</pre>
    </section>

    <section class="panel">
      <h3>回放 (CSV → 总线)</h3>
      <div class="row">
        <label>输入路径</label><input v-model="repPath" style="width: 240px" />
        <label>速度(倍)</label>
        <input v-model.number="repSpeed" type="number" step="0.1" style="width: 80px" />
        <button @click="doReplay">开始回放</button>
      </div>
      <pre>{{ repResult || '待回放' }}</pre>
    </section>

    <section class="panel grow">
      <h3>离线分析</h3>
      <div class="row">
        <label>CSV 路径</label><input v-model="anaPath" style="width: 240px" />
        <label>波特率</label><input v-model.number="anaBitrate" style="width: 100px" />
        <button @click="doAnalyze">分析</button>
      </div>
      <div v-if="analysis" class="row" style="flex-wrap: wrap; gap: 16px">
        <span>总帧: <b>{{ analysis.total_frames }}</b></span>
        <span>FD: <b>{{ analysis.fd_frames }}</b></span>
        <span>跨度: <b>{{ analysis.span_ms }} ms</b></span>
        <span>负载率: <b>{{ (analysis.load_permille / 10).toFixed(1) }}%</b></span>
        <span>平均间隔: <b>{{ analysis.avg_interval_ms.toFixed(2) }} ms</b></span>
      </div>
      <div class="table-wrap" v-if="analysis" style="max-height: 200px; margin-top: 8px">
        <table>
          <thead><tr><th>Top ID</th><th>帧数</th></tr></thead>
          <tbody>
            <tr v-for="(pair, i) in analysis.top_ids" :key="i">
              <td class="mono">0x{{ pair[0].toString(16).toUpperCase() }}</td>
              <td>{{ pair[1] }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <section class="panel grow">
      <h3>说明</h3>
      <div class="log">
        录制将周期性订阅总线帧并写入 CSV（timestamp,type,id,brs,esi,dlc,data）。
        回放按原始时间间隔 × 速度因子重新发送到总线（0.5=两倍速，2.0=半速）。
        离线分析读取 CSV 并计算帧数、时间跨度、总线负载率与 Top 节点。
        BLF/ASC 格式为后续扩展项。
      </div>
    </section>
  </div>
</template>
