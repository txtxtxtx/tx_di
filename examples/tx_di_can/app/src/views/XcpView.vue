<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/can'
import { pushLog } from '../store'
import { t } from '../i18n'
import type { XcpA2lInfo, XcpValue } from '../types'

const a2lPath = ref('example.a2l')
const info = ref<XcpA2lInfo | null>(null)
const values = ref<XcpValue[]>([])
const daqName = ref('')
const daqValue = ref<XcpValue | null>(null)
const calName = ref('')
const calData = ref('01 02')

function parseHex(s: string): number[] {
  return s.trim().split(/\s+/).filter(Boolean).map((h) => parseInt(h, 16))
}
async function doParse() {
  try {
    info.value = await api.xcpParseA2l(a2lPath.value)
    pushLog(`解析 A2L: ${info.value.module} / ${info.value.measurements.length} 测量 / ${info.value.characteristics.length} 标定`)
  } catch (e) {
    pushLog('A2L 解析失败: ' + String(e))
  }
}
async function doMeasure() {
  try {
    values.value = await api.xcpMeasureAll(a2lPath.value)
    pushLog(`测量 ${values.value.length} 个量`)
  } catch (e) {
    pushLog('测量失败: ' + String(e))
  }
}
async function doDaq() {
  if (!daqName.value) return
  try {
    daqValue.value = await api.xcpDaqSample(a2lPath.value, daqName.value)
    pushLog(`DAQ 采样 ${daqName.value}: ${daqValue.value.hex}`)
  } catch (e) {
    pushLog('DAQ 失败: ' + String(e))
  }
}
async function doCalibrate() {
  if (!calName.value) return
  try {
    await api.xcpCalibrate(a2lPath.value, calName.value, parseHex(calData.value))
    pushLog(`标定写入 ${calName.value}`)
  } catch (e) {
    pushLog('标定失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <h3>{{ t('title.xcp') }}</h3>
    <section class="panel">
      <div class="row">
        <label>A2L 路径</label><input v-model="a2lPath" style="width: 240px" />
        <button @click="doParse">解析</button>
        <button @click="doMeasure">测量全部</button>
      </div>
    </section>

    <div style="display: flex; gap: 12px; flex: 1; min-height: 0">
      <section class="panel grow">
        <h3>测量量 ({{ info?.measurements.length ?? 0 }})</h3>
        <div class="table-wrap" style="max-height: 240px">
          <table>
            <thead><tr><th>名称</th><th>类型</th><th>地址</th><th>单位</th><th>当前值</th></tr></thead>
            <tbody>
              <tr v-for="m in info?.measurements ?? []" :key="m.name">
                <td>{{ m.name }}</td>
                <td class="mono">{{ m.datatype }}</td>
                <td class="mono">0x{{ m.address.toString(16).toUpperCase() }}</td>
                <td>{{ m.unit }}</td>
                <td class="mono">{{ values.find((v) => v.name === m.name)?.hex ?? '-' }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="panel grow">
        <h3>标定量 ({{ info?.characteristics.length ?? 0 }})</h3>
        <div class="table-wrap" style="max-height: 240px">
          <table>
            <thead><tr><th>名称</th><th>类型</th><th>地址</th><th>单位</th></tr></thead>
            <tbody>
              <tr v-for="c in info?.characteristics ?? []" :key="c.name">
                <td>{{ c.name }}</td>
                <td class="mono">{{ c.datatype }}</td>
                <td class="mono">0x{{ c.address.toString(16).toUpperCase() }}</td>
                <td>{{ c.unit }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>

    <section class="panel">
      <h3>DAQ 采样</h3>
      <div class="row">
        <label>测量量名</label><input v-model="daqName" style="width: 160px" />
        <button @click="doDaq">采样</button>
        <span class="mono" v-if="daqValue">→ {{ daqValue.hex }}</span>
      </div>
    </section>

    <section class="panel">
      <h3>标定写入 (DOWNLOAD)</h3>
      <div class="row">
        <label>标定量名</label><input v-model="calName" style="width: 160px" />
        <label>数据(hex)</label><input v-model="calData" style="width: 160px" />
        <button @click="doCalibrate">写入</button>
      </div>
    </section>
  </div>
</template>
