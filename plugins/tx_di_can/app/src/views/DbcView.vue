<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/can'
import { pushLog } from '../store'
import type { DbcMsgInfo, DbcValue } from '../types'

const dbcPath = ref('example.dbc')
const messages = ref<DbcMsgInfo[]>([])
const decodeId = ref('100')
const decodeData = ref('80 00 00 00 00 00 00 00')
const values = ref<DbcValue[]>([])

function parseData(s: string): number[] {
  return s.trim().split(/\s+/).filter(Boolean).map((h) => parseInt(h, 16))
}
async function doLoad() {
  try {
    messages.value = await api.loadDbc(dbcPath.value)
    pushLog(`加载 DBC: ${messages.value.length} 条消息`)
  } catch (e) {
    pushLog('DBC 加载失败: ' + String(e))
  }
}
async function doDecode() {
  try {
    values.value = await api.decodeDbc(
      dbcPath.value,
      parseInt(decodeId.value, 16),
      parseData(decodeData.value),
    )
    pushLog(`解码 ${values.value.length} 个信号`)
  } catch (e) {
    pushLog('解码失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <section class="panel">
      <h3>DBC 加载</h3>
      <div class="row">
        <label>文件路径</label><input v-model="dbcPath" style="width: 240px" />
        <button @click="doLoad">加载</button>
      </div>
    </section>

    <div style="display: flex; gap: 12px; flex: 1; min-height: 0">
      <section class="panel grow">
        <h3>消息 ({{ messages.length }})</h3>
        <div class="table-wrap" style="max-height: 260px">
          <table>
            <thead><tr><th>ID</th><th>名称</th><th>DLC</th><th>信号数</th></tr></thead>
            <tbody>
              <tr v-for="m in messages" :key="m.id">
                <td class="mono">0x{{ m.id.toString(16).toUpperCase() }}</td>
                <td>{{ m.name }}</td>
                <td>{{ m.dlc }}</td>
                <td>{{ m.signals.length }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="panel grow">
        <h3>信号解码</h3>
        <div class="row">
          <label>CAN ID(hex)</label><input v-model="decodeId" style="width: 80px" />
          <label>数据(hex)</label><input v-model="decodeData" style="width: 240px" />
          <button @click="doDecode">解码</button>
        </div>
        <div class="table-wrap" style="max-height: 220px">
          <table>
            <thead><tr><th>信号</th><th>值</th></tr></thead>
            <tbody>
              <tr v-for="v in values" :key="v.name">
                <td>{{ v.name }}</td>
                <td class="mono">{{ v.value }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>
  </div>
</template>
