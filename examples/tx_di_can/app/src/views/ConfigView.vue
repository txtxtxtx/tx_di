<script setup lang="ts">
import { reactive } from 'vue'
import { api } from '../api/can'
import { state, pushLog } from '../store'
import type { CanConfig } from '../types'

const form = reactive<CanConfig>({
  adapter: 'simbus',
  interface: 'vcan0',
  bitrate: 500000,
  fd_bitrate: 2000000,
  enable_fd: false,
  rx_queue_size: 512,
  tx_timeout_ms: 100,
  isotp_tx_id: 0x7e0,
  isotp_rx_id: 0x7e8,
  isotp_block_size: 0,
  isotp_st_min_ms: 0,
  uds_p2_timeout_ms: 150,
  uds_p2_star_timeout_ms: 5000,
})

async function doConnect() {
  try {
    await api.connect({ ...form })
    state.connected = true
    state.config = { ...form }
    pushLog('已连接')
  } catch (e) {
    pushLog('连接失败: ' + String(e))
  }
}
async function doDisconnect() {
  try {
    await api.disconnect()
    state.connected = false
    pushLog('已断开')
  } catch (e) {
    pushLog('断开失败: ' + String(e))
  }
}
async function loadDefault() {
  try {
    const c = await api.defaultConfig()
    Object.assign(form, c)
  } catch (e) {
    pushLog('加载默认配置失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <section class="panel">
      <h3>连接配置</h3>
      <div class="row">
        <label>适配器</label>
        <select v-model="form.adapter">
          <option value="simbus">simbus (仿真)</option>
          <option value="socketcan">socketcan (Linux)</option>
          <option value="pcan">pcan (PEAK, Windows)</option>
        </select>
        <label>接口</label><input v-model="form.interface" style="width: 120px" />
        <label>比特率</label><input v-model.number="form.bitrate" style="width: 100px" />
        <label>CAN-FD</label><input type="checkbox" v-model="form.enable_fd" />
      </div>
      <div class="row">
        <label>ISO-TP Tx ID</label><input v-model.number="form.isotp_tx_id" style="width: 90px" />
        <label>ISO-TP Rx ID</label><input v-model.number="form.isotp_rx_id" style="width: 90px" />
        <label>FD 比特率</label><input v-model.number="form.fd_bitrate" style="width: 100px" />
        <label>接收队列</label><input v-model.number="form.rx_queue_size" style="width: 80px" />
      </div>
      <div class="row">
        <button @click="doConnect">连接</button>
        <button @click="doDisconnect">断开</button>
        <button @click="loadDefault">加载默认</button>
      </div>
    </section>

    <section class="panel grow">
      <h3>日志</h3>
      <div class="log">{{ state.log.join('\n') }}</div>
    </section>
  </div>
</template>
