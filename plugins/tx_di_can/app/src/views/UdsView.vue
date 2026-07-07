<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/can'
import { state, pushLog } from '../store'

const txId = ref('7E0')
const did = ref('F189')
const readResult = ref('')
const writeData = ref('')
const session = ref('1')
const resetType = ref('1')
const secLevel = ref('1')
const keyAlgo = ref('negate')
const dtcMask = ref('FF')
const dtcResult = ref('')

function tx() {
  return parseInt(txId.value, 16)
}
function parseData(s: string): number[] {
  return s.trim().split(/\s+/).filter(Boolean).map((h) => parseInt(h, 16))
}
const hex = (n: number) => n.toString(16).padStart(2, '0').toUpperCase()

async function doRead() {
  try {
    const d = parseInt(did.value, 16)
    readResult.value = await api.readData(tx(), d)
    pushLog(`读 DID 0x${hex(d)}: ${readResult.value}`)
  } catch (e) {
    pushLog('读失败: ' + String(e))
  }
}
async function doWrite() {
  try {
    const d = parseInt(did.value, 16)
    await api.writeData(tx(), d, parseData(writeData.value))
    pushLog(`写 DID 0x${hex(d)} 成功`)
  } catch (e) {
    pushLog('写失败: ' + String(e))
  }
}
async function doSession() {
  try {
    await api.sessionControl(tx(), parseInt(session.value, 16))
    pushLog('切换会话成功')
  } catch (e) {
    pushLog('失败: ' + String(e))
  }
}
async function doReset() {
  try {
    await api.ecuReset(tx(), parseInt(resetType.value, 16))
    pushLog('复位指令已发送')
  } catch (e) {
    pushLog('失败: ' + String(e))
  }
}
async function doTester() {
  try {
    await api.testerPresent(tx())
    pushLog('TesterPresent OK')
  } catch (e) {
    pushLog('失败: ' + String(e))
  }
}
async function doSec() {
  try {
    await api.securityAccess(tx(), parseInt(secLevel.value, 16), keyAlgo.value)
    pushLog('安全访问成功')
  } catch (e) {
    pushLog('失败: ' + String(e))
  }
}
async function doDtc() {
  try {
    const r = await api.readDtc(tx(), parseInt(dtcMask.value, 16))
    dtcResult.value = r.join('\n')
    pushLog(`读到 ${r.length} 条 DTC`)
  } catch (e) {
    pushLog('失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <section class="panel">
      <h3>会话 / 控制</h3>
      <div class="row">
        <label>Tx ID(hex)</label><input v-model="txId" style="width: 90px" />
        <label>会话</label>
        <select v-model="session">
          <option value="1">0x01 默认</option>
          <option value="2">0x02 编程</option>
          <option value="3">0x03 扩展</option>
        </select>
        <button @click="doSession">切换会话</button>
        <label>复位类型</label>
        <input v-model="resetType" style="width: 50px" />
        <button @click="doReset">ECU 复位</button>
        <button @click="doTester">TesterPresent</button>
      </div>
      <div class="row">
        <label>安全等级</label><input v-model="secLevel" style="width: 50px" />
        <label>密钥算法</label>
        <select v-model="keyAlgo">
          <option value="negate">取反(negate)</option>
          <option value="none">原样(none)</option>
        </select>
        <button @click="doSec">安全访问</button>
      </div>
    </section>

    <section class="panel">
      <h3>读写 DID (0x22 / 0x2E)</h3>
      <div class="row">
        <label>DID(hex)</label><input v-model="did" style="width: 80px" />
        <button @click="doRead">读取</button>
        <label>写入数据(hex)</label><input v-model="writeData" style="width: 220px" />
        <button @click="doWrite">写入</button>
      </div>
    </section>

    <section class="panel">
      <h3>DTC 读取 (0x19)</h3>
      <div class="row">
        <label>状态掩码(hex)</label><input v-model="dtcMask" style="width: 60px" />
        <button @click="doDtc">读取 DTC</button>
      </div>
    </section>

    <div style="display: flex; gap: 12px; flex: 1; min-height: 0">
      <section class="panel grow">
        <h3>读取结果</h3>
        <pre>{{ readResult }}</pre>
      </section>
      <section class="panel grow">
        <h3>DTC 列表</h3>
        <pre>{{ dtcResult }}</pre>
      </section>
    </div>

    <section class="panel">
      <h3>日志</h3>
      <div class="log">{{ state.log.join('\n') }}</div>
    </section>
  </div>
</template>
