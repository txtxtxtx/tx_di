<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/can'
import { pushLog } from '../store'

const recPath = ref('record.csv')
const recDuration = ref(3000)
const recResult = ref('')

const repPath = ref('record.csv')
const repSpeed = ref(1.0)
const repResult = ref('')

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
      <h3>说明</h3>
      <div class="log">
        录制将周期性订阅总线帧并写入 CSV（timestamp,type,id,brs,esi,dlc,data）。
        回放按原始时间间隔 × 速度因子重新发送到总线（0.5=两倍速，2.0=半速）。
        BLF/ASC 格式为后续扩展项。
      </div>
    </section>
  </div>
</template>
