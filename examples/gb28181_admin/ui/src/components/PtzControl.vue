<template>
  <div class="ptz-panel">
    <!-- 方向按钮 -->
    <div class="ptz-grid">
      <button class="ptz-btn corner" @mousedown="start('upleft')"    @mouseup="stop" @mouseleave="stop">↖</button>
      <button class="ptz-btn"        @mousedown="start('up')"        @mouseup="stop" @mouseleave="stop">▲</button>
      <button class="ptz-btn corner" @mousedown="start('upright')"   @mouseup="stop" @mouseleave="stop">↗</button>
      <button class="ptz-btn"        @mousedown="start('left')"      @mouseup="stop" @mouseleave="stop">◀</button>
      <button class="ptz-btn ptz-stop" @click="stop">■</button>
      <button class="ptz-btn"        @mousedown="start('right')"     @mouseup="stop" @mouseleave="stop">▶</button>
      <button class="ptz-btn corner" @mousedown="start('downleft')"  @mouseup="stop" @mouseleave="stop">↙</button>
      <button class="ptz-btn"        @mousedown="start('down')"      @mouseup="stop" @mouseleave="stop">▼</button>
      <button class="ptz-btn corner" @mousedown="start('downright')" @mouseup="stop" @mouseleave="stop">↘</button>
    </div>

    <!-- 变倍 + 速度 -->
    <div class="ptz-extra">
      <div class="zoom-btns">
        <button class="ptz-btn ptz-zoom" @mousedown="start('zoomin')"  @mouseup="stop" @mouseleave="stop">🔍+</button>
        <button class="ptz-btn ptz-zoom" @mousedown="start('zoomout')" @mouseup="stop" @mouseleave="stop">🔍−</button>
      </div>
      <div class="speed-ctrl">
        <label>速度</label>
        <input type="range" v-model.number="speed" min="10" max="255" step="5" />
        <span>{{ speed }}</span>
      </div>
    </div>

    <div v-if="status" :class="['ptz-status', statusOk ? 'ok' : 'err']">{{ status }}</div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import api from '../api/index.js'

const props = defineProps({
  deviceId:  { type: String, required: true },
  channelId: { type: String, required: true },
})

const speed  = ref(64)
const status = ref('')
const statusOk = ref(true)
let holding = false

async function start(direction) {
  holding = true
  try {
    await api.ptz(props.deviceId, {
      channel_id: props.channelId,
      direction,
      pan:  speed.value,
      tilt: speed.value,
      zoom: speed.value,
    })
    status.value  = `${direction} 已发送`
    statusOk.value = true
  } catch (e) {
    status.value  = e.message
    statusOk.value = false
  }
}

async function stop() {
  if (!holding) return
  holding = false
  try {
    await api.ptz(props.deviceId, {
      channel_id: props.channelId,
      direction: 'stop',
      pan: 0, tilt: 0, zoom: 0,
    })
    status.value  = '已停止'
    statusOk.value = true
  } catch {}
}
</script>

<style scoped>
.ptz-panel { user-select: none; }
.ptz-grid {
  display: grid; grid-template-columns: repeat(3, 48px);
  gap: 4px; margin-bottom: 10px;
}
.ptz-btn {
  width: 48px; height: 48px; border: 1px solid var(--border);
  border-radius: 8px; background: var(--card-bg);
  font-size: 16px; transition: all .1s;
  display: flex; align-items: center; justify-content: center;
}
.ptz-btn:hover  { background: var(--bg); border-color: var(--primary); color: var(--primary); }
.ptz-btn:active { background: #e8f0fe; }
.ptz-btn.corner { font-size: 14px; color: var(--text-muted); }
.ptz-stop { background: #fff1f2; border-color: var(--danger); color: var(--danger); font-size: 13px; }

.ptz-extra { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; margin-top: 4px; }
.zoom-btns { display: flex; gap: 4px; }
.ptz-zoom  { width: auto; padding: 0 10px; font-size: 13px; }
.speed-ctrl {
  display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-muted);
}
.speed-ctrl input { width: 90px; accent-color: var(--primary); }
.speed-ctrl span  { min-width: 28px; font-weight: 500; color: var(--text); }

.ptz-status { margin-top: 8px; font-size: 12px; padding: 4px 8px; border-radius: 4px; }
.ok  { color: #16a34a; background: #dcfce7; }
.err { color: var(--danger); background: #fee2e2; }
</style>
