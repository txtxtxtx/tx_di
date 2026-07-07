<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '../api/can'
import { pushLog } from '../store'
import { t } from '../i18n'
import type { CanConfig, ProjectConfig } from '../types'

const path = ref('demo.canproj')
const cfg = ref<ProjectConfig>({
  name: '未命名工程',
  can: {
    adapter: 'simbus',
    interface: 'vcan0',
    bitrate: 500000,
    fd_bitrate: 2000000,
    enable_fd: false,
    rx_queue_size: 512,
    sim_ecu: false,
    tx_timeout_ms: 100,
    isotp_tx_id: 0x7e0,
    isotp_rx_id: 0x7e8,
    isotp_block_size: 0,
    isotp_st_min_ms: 0,
    uds_p2_timeout_ms: 150,
    uds_p2_star_timeout_ms: 5000,
  } as CanConfig,
  uds_tx_id: 0x7e0,
  recent_dids: [0xf190, 0xf195, 0xf18c],
  recent_dtcs: [],
  flash: {
    target_id: 0x7e0,
    security_level: 0x01,
    memory_address: 0x08000000,
    erase_before_download: false,
  },
})

const didText = ref(cfg.value.recent_dids.map((d) => '0x' + d.toString(16)).join(', '))

onMounted(async () => {
  try {
    cfg.value.can = await api.defaultConfig()
  } catch {
    /* 使用内置默认 */
  }
})

async function doSave() {
  cfg.value.recent_dids = didText.value
    .split(',')
    .map((s) => parseInt(s.trim().replace(/^0x/i, ''), 16))
    .filter((n) => !isNaN(n))
  try {
    await api.saveProject(path.value, cfg.value)
    pushLog(`已保存工程: ${path.value}`)
  } catch (e) {
    pushLog('保存失败: ' + String(e))
  }
}
async function doLoad() {
  try {
    const p = await api.loadProject(path.value)
    cfg.value = p
    didText.value = p.recent_dids.map((d) => '0x' + d.toString(16)).join(', ')
    pushLog(`已加载工程: ${p.name}`)
  } catch (e) {
    pushLog('加载失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <h3>{{ t('title.project') }}</h3>
    <section class="panel">
      <div class="row">
        <label>工程路径</label><input v-model="path" style="width: 220px" />
        <button @click="doLoad">{{ t('common.load') }}</button>
        <button @click="doSave">{{ t('common.save') }}</button>
      </div>
    </section>

    <div style="display: flex; gap: 12px; flex: 1; min-height: 0">
      <section class="panel grow">
        <h3>总线 / 适配器</h3>
        <div class="row"><label>工程名</label><input v-model="cfg.name" style="width: 200px" /></div>
        <div class="row">
          <label>适配器</label>
          <select v-model="cfg.can.adapter">
            <option value="simbus">SimBus</option>
            <option value="pcan">PCAN</option>
            <option value="socketcan">SocketCAN</option>
            <option value="kvaser">Kvaser</option>
          </select>
        </div>
        <div class="row"><label>接口</label><input v-model="cfg.can.interface" style="width: 120px" /></div>
        <div class="row"><label>波特率</label><input v-model.number="cfg.can.bitrate" style="width: 100px" /></div>
        <div class="row"><label>FD</label><input type="checkbox" v-model="cfg.can.enable_fd" /></div>
      </section>

      <section class="panel grow">
        <h3>诊断 / 刷写</h3>
        <div class="row"><label>UDS TX ID</label><input v-model.number="cfg.uds_tx_id" style="width: 100px" /></div>
        <div class="row"><label>目标 ID</label><input v-model.number="cfg.flash.target_id" style="width: 100px" /></div>
        <div class="row"><label>安全等级</label><input v-model.number="cfg.flash.security_level" style="width: 70px" /></div>
        <div class="row"><label>内存地址</label><input v-model.number="cfg.flash.memory_address" style="width: 120px" /></div>
        <div class="row"><label>显式擦除</label><input type="checkbox" v-model="cfg.flash.erase_before_download" /></div>
        <div class="row"><label>常用 DID</label><input v-model="didText" style="width: 220px" /></div>
      </section>
    </div>
  </div>
</template>
