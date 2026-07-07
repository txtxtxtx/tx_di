<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue'
import { api } from '../api/can'
import { state, pushLog, refreshStats } from '../store'
import type { FrameFilter } from '../types'

const sendId = ref('7E0')
const sendData = ref('')
const isFd = ref(false)
const brs = ref(true)
const esi = ref(false)
const loop = ref(false)
const loopMs = ref(1000)
let loopTimer: number | null = null

const fMin = ref('')
const fMax = ref('')
const fMask = ref('')
const fMatch = ref('')

const decode = ref<'hex' | 'dec' | 'bin' | 'ascii'>('hex')
const search = ref('')
const frozen = ref(false)

// 负载率迷你曲线：保留最近 40 个采样点
const loadSeries = ref<number[]>([])
let statTimer: number | null = null

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
function toggleLoop() {
  loop.value = !loop.value
  if (loop.value) {
    loopTimer = window.setInterval(doSend, Math.max(50, loopMs.value))
    pushLog(`循环发送已开启 (${loopMs.value}ms)`)
  } else if (loopTimer) {
    clearInterval(loopTimer)
    loopTimer = null
    pushLog('循环发送已停止')
  }
}

function parseNum(s: string): number | null {
  const t = s.trim()
  if (!t) return null
  return parseInt(t, 16)
}
async function applyFilter() {
  const filter: FrameFilter = {
    id_min: parseNum(fMin.value),
    id_max: parseNum(fMax.value),
    id_mask: parseNum(fMask.value) ?? 0,
    id_match: parseNum(fMatch.value) ?? 0,
  }
  const empty =
    filter.id_min === null &&
    filter.id_max === null &&
    filter.id_mask === 0
  try {
    await api.setFrameFilter(empty ? null : filter)
    state.filter = empty ? null : filter
    pushLog(empty ? '已清除过滤器' : '已应用过滤器')
  } catch (e) {
    pushLog('过滤器设置失败: ' + String(e))
  }
}
async function clearFilter() {
  fMin.value = ''; fMax.value = ''; fMask.value = ''; fMatch.value = ''
  await applyFilter()
}

function decodeCell(raw: number[]): string {
  switch (decode.value) {
    case 'dec': return raw.join(' ')
    case 'bin': return raw.map((b) => b.toString(2).padStart(8, '0')).join(' ')
    case 'ascii':
      return raw.map((b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : '.')).join('')
    default: return raw.map((b) => b.toString(16).padStart(2, '0')).join(' ')
  }
}

const filteredRows = computed(() => {
  const q = search.value.trim().toLowerCase()
  return state.trace.filter((r) => {
    if (frozen.value) return true
    if (!q) return true
    return (
      r.dataHex.toLowerCase().includes(q) ||
      r.info.toLowerCase().includes(q) ||
      r.type.toLowerCase().includes(q)
    )
  })
})

function rowHighlight(r: { frameId: number }): boolean {
  const f = state.filter
  if (!f) return false
  if (f.id_min !== null && r.frameId < f.id_min) return false
  if (f.id_max !== null && r.frameId > f.id_max) return false
  if (f.id_mask && (r.frameId & f.id_mask) !== (f.id_match & f.id_mask)) return false
  return true
}

function exportCsv() {
  const header = '#,时间,类型,方向,ID,数据\n'
  const body = state.trace
    .map(
      (r) =>
        `${r.id},${new Date(r.ts).toLocaleTimeString()},${r.type},${r.dir},0x${r.frameId
          .toString(16)
          .toUpperCase()},${r.dataHex}`,
    )
    .join('\n')
  const blob = new Blob([header + body], { type: 'text/csv' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `trace_${Date.now()}.csv`
  a.click()
  URL.revokeObjectURL(url)
  pushLog('已导出 CSV')
}
function clearTrace() {
  state.trace.splice(0, state.trace.length)
}

function poll() {
  refreshStats()
  loadSeries.value.push(state.stats.load_permille)
  if (loadSeries.value.length > 40) loadSeries.value.shift()
}

onMounted(() => {
  statTimer = window.setInterval(poll, 500)
})
onUnmounted(() => {
  if (statTimer) clearInterval(statTimer)
  if (loopTimer) clearInterval(loopTimer)
})
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
        <label><input type="checkbox" v-model="loop" @change="toggleLoop" /> 循环</label>
        <input v-model.number="loopMs" type="number" style="width: 80px" /> ms
      </div>
    </section>

    <section class="panel">
      <h3>过滤器 / 掩码</h3>
      <div class="row">
        <label>ID≥</label><input v-model="fMin" style="width: 90px" placeholder="hex" />
        <label>ID≤</label><input v-model="fMax" style="width: 90px" placeholder="hex" />
        <label>掩码</label><input v-model="fMask" style="width: 90px" placeholder="hex" />
        <label>匹配</label><input v-model="fMatch" style="width: 90px" placeholder="hex" />
        <button @click="applyFilter">应用</button>
        <button @click="clearFilter">清除</button>
        <span class="muted">
          {{
            state.filter
              ? `生效中: ${state.filter.id_min ? '≥' + state.filter.id_min.toString(16) : ''} ${
                  state.filter.id_max ? '≤' + state.filter.id_max.toString(16) : ''
                }`
              : '不过滤'
          }}
        </span>
      </div>
    </section>

    <section class="panel">
      <h3>总线统计</h3>
      <div class="row stats">
        <div class="stat"><span class="stat-val">{{ state.stats.frame_count }}</span><span class="stat-label">标准帧</span></div>
        <div class="stat"><span class="stat-val">{{ state.stats.fd_frame_count }}</span><span class="stat-label">FD帧</span></div>
        <div class="stat"><span class="stat-val">{{ state.stats.bytes }}</span><span class="stat-label">字节</span></div>
        <div class="stat">
          <span class="stat-val">{{ (state.stats.load_permille / 10).toFixed(1) }}%</span>
          <span class="stat-label">总线负载</span>
        </div>
        <div class="loadbar">
          <div
            v-for="(v, i) in loadSeries"
            :key="i"
            class="loadcell"
            :style="{ height: Math.max(2, (v / 1000) * 100) + '%' }"
          ></div>
        </div>
        <button @click="api.resetStats()">重置计数</button>
      </div>
    </section>

    <section class="panel grow">
      <h3>
        实时报文 ({{ state.trace.length }})
        <span class="toolbar">
          <select v-model="decode">
            <option value="hex">HEX</option>
            <option value="dec">DEC</option>
            <option value="bin">BIN</option>
            <option value="ascii">ASCII</option>
          </select>
          <input v-model="search" placeholder="查找 ID/数据/类型" style="width: 160px" />
          <label><input type="checkbox" v-model="frozen" /> 冻结</label>
          <button @click="exportCsv">导出CSV</button>
          <button @click="clearTrace">清空</button>
        </span>
      </h3>
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>#</th><th>时间</th><th>类型</th><th>方向</th><th>ID</th><th>数据</th></tr>
          </thead>
          <tbody>
            <tr
              v-for="r in filteredRows"
              :key="r.id"
              :class="{ tx: r.dir === 'TX', rx: r.dir === 'RX', hl: rowHighlight(r) }"
            >
              <td>{{ r.id }}</td>
              <td>{{ new Date(r.ts).toLocaleTimeString() }}</td>
              <td>{{ r.type }}</td>
              <td>{{ r.dir }}</td>
              <td class="mono">0x{{ r.frameId.toString(16).toUpperCase() }}</td>
              <td class="mono">{{ decodeCell(r.dataRaw) }}</td>
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
