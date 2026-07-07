<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/can'
import { state, pushLog } from '../store'

const sendId = ref('7E0')
const sendData = ref('')
const isFd = ref(false)
const brs = ref(true)
const esi = ref(false)

function parseData(): number[] {
  return sendData.value
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((h) => parseInt(h, 16))
}
async function doSend() {
  const id = parseInt(sendId.value, 16)
  const data = parseData()
  try {
    if (isFd.value) await api.sendFdFrame({ id, data, brs: brs.value, esi: esi.value })
    else await api.sendFrame({ id, data })
    pushLog(`已发送 0x${id.toString(16)} [${data.length}]`)
  } catch (e) {
    pushLog('发送失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <section class="panel">
      <h3>发送帧</h3>
      <div class="row">
        <label>ID(hex)</label>
        <input v-model="sendId" style="width: 100px" />
        <label>数据(hex, 空格分隔)</label>
        <input v-model="sendData" style="width: 340px" />
      </div>
      <div class="row">
        <label><input type="checkbox" v-model="isFd" /> CAN-FD</label>
        <label v-if="isFd"><input type="checkbox" v-model="brs" /> BRS</label>
        <label v-if="isFd"><input type="checkbox" v-model="esi" /> ESI</label>
        <button @click="doSend">发送</button>
      </div>
    </section>

    <section class="panel grow">
      <h3>实时报文 ({{ state.trace.length }})</h3>
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>#</th><th>时间</th><th>类型</th><th>方向</th><th>ID/信息</th><th>数据</th></tr>
          </thead>
          <tbody>
            <tr v-for="r in state.trace" :key="r.id">
              <td>{{ r.id }}</td>
              <td>{{ new Date(r.ts).toLocaleTimeString() }}</td>
              <td>{{ r.type }}</td>
              <td>{{ r.dir }}</td>
              <td class="mono">{{ r.info }}</td>
              <td class="mono">{{ r.dataHex }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <section class="panel">
      <h3>日志</h3>
      <div class="log">{{ state.log.join('\n') }}</div>
    </section>
  </div>
</template>
