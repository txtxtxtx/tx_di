<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/can'
import { state, pushLog } from '../store'

const firmwarePath = ref('firmware.bin')
const targetId = ref('7E0')
const memoryAddress = ref('08000000')
const securityLevel = ref('1')
const eraseBefore = ref(false)
const keyAlgo = ref('negate')

const pct = () =>
  state.flash.totalBytes > 0
    ? Math.round((state.flash.bytesSent / state.flash.totalBytes) * 100)
    : 0

function onPick(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0]
  // 注：WebView2 下标准 <input type=file> 仅提供文件名；真实路径需经 Tauri 文件对话框
  if (f) firmwarePath.value = f.name
}

async function doFlash() {
  if (state.flash.active) return
  state.flash.active = true
  state.flash.status = '刷写中...'
  state.flash.bytesSent = 0
  state.flash.totalBytes = 0
  try {
    await api.flash(
      firmwarePath.value,
      {
        target_id: parseInt(targetId.value, 16),
        memory_address: parseInt(memoryAddress.value, 16),
        security_level: parseInt(securityLevel.value, 16),
        erase_before_download: eraseBefore.value,
      },
      keyAlgo.value,
    )
    pushLog('刷写命令完成')
  } catch (e) {
    state.flash.active = false
    state.flash.status = '失败: ' + String(e)
    pushLog('刷写失败: ' + String(e))
  }
}
</script>

<template>
  <div class="view">
    <section class="panel">
      <h3>固件刷写 (UDS 0x34~0x37)</h3>
      <div class="row">
        <label>固件文件</label>
        <input :value="firmwarePath" style="width: 240px" readonly />
        <input type="file" accept=".bin,.s19,.srec,.hex" @change="onPick" style="width: 220px" />
      </div>
      <div class="row">
        <label>Target ID(hex)</label><input v-model="targetId" style="width: 90px" />
        <label>内存地址(hex)</label><input v-model="memoryAddress" style="width: 130px" />
        <label>安全等级(hex)</label><input v-model="securityLevel" style="width: 60px" />
      </div>
      <div class="row">
        <label><input type="checkbox" v-model="eraseBefore" /> 下载前显式擦除 (0x31 0xFF00)</label>
        <label>密钥算法</label>
        <select v-model="keyAlgo">
          <option value="negate">取反(negate)</option>
          <option value="none">原样(none)</option>
        </select>
        <button :disabled="state.flash.active" @click="doFlash">开始刷写</button>
      </div>
    </section>

    <section class="panel">
      <h3>进度</h3>
      <div class="progress">
        <div :style="{ width: pct() + '%' }"></div>
      </div>
      <div class="row" style="margin-top: 8px">
        <span :class="{ err: state.flash.status.startsWith('失败') }">
          {{ state.flash.bytesSent }} / {{ state.flash.totalBytes }} 字节 ·
          块 {{ state.flash.blockSeq }} / {{ state.flash.totalBlocks }} ·
          {{ state.flash.status }}
        </span>
      </div>
    </section>

    <section class="panel grow">
      <h3>日志</h3>
      <div class="log">{{ state.log.join('\n') }}</div>
    </section>
  </div>
</template>

<style scoped>
.err { color: var(--err); font-weight: 600; }
</style>
