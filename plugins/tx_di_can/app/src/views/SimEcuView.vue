<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api/can'
import { pushLog } from '../store'

const enabled = ref(false)
const txId = ref('7E0')
const selfTest = ref('')
const supportedSids = [
  '0x10 诊断会话', '0x11 ECU 复位', '0x14 清除 DTC', '0x19 读 DTC',
  '0x22 读 DID', '0x23 读内存', '0x27 安全访问', '0x2E 写 DID',
  '0x31 例程控制', '0x34 请求下载', '0x36 传输数据', '0x37 退出传输', '0x3E 保活',
]

async function refresh() {
  enabled.value = await api.simEcuStatus()
}
async function doSelfTest() {
  const tx = parseInt(txId.value, 16)
  selfTest.value = ''
  try {
    const vin = await api.readData(tx, 0xf190)
    selfTest.value += `VIN: ${vin}\n`
    const dtcs = await api.readDtc(tx, 0xff)
    selfTest.value += `DTC(${dtcs.length}):\n${dtcs.join('\n')}\n`
    selfTest.value += '自检通过 ✓'
    pushLog('ECU 仿真自检通过')
  } catch (e) {
    selfTest.value = '自检失败: ' + String(e)
    pushLog('自检失败: ' + String(e))
  }
}

onMounted(refresh)
</script>

<template>
  <div class="view">
    <section class="panel">
      <h3>ECU 仿真节点</h3>
      <div class="row">
        <span class="stat-val" :style="{ color: enabled ? 'var(--ok)' : 'var(--err)' }">
          {{ enabled ? '运行中' : '未启用' }}
        </span>
        <span class="muted">
          使用 SimBus 适配器或开启 sim_ecu 时自动启动；按描述库自动应答 UDS 请求
        </span>
        <button @click="refresh">刷新状态</button>
      </div>
      <div class="row">
        <label>诊断 Tx ID(hex)</label><input v-model="txId" style="width: 90px" />
        <button @click="doSelfTest">一键自检</button>
      </div>
    </section>

    <section class="panel">
      <h3>支持的 UDS 服务</h3>
      <div class="row" style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px">
        <span v-for="s in supportedSids" :key="s" class="chip">{{ s }}</span>
      </div>
    </section>

    <section class="panel grow">
      <h3>自检结果</h3>
      <pre>{{ selfTest || '尚未运行自检' }}</pre>
    </section>
  </div>
</template>

<style scoped>
.chip {
  background: var(--panel2);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 4px 8px;
  font-size: 12px;
  color: var(--muted);
}
</style>
