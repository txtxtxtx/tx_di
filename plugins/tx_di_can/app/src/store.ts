// 全局响应式状态
import { reactive } from 'vue'
import type { CanConfig, CanEvent } from './types'

export interface TraceRow {
  id: number
  ts: number
  type: string
  dir: string
  dataHex: string
  info: string
}

interface FrameLike {
  id: { Standard?: number; Extended?: number }
  data: number[]
}

export const state = reactive({
  connected: false,
  config: null as CanConfig | null,
  trace: [] as TraceRow[],
  log: [] as string[],
  flash: {
    active: false,
    blockSeq: 0,
    totalBlocks: 0,
    bytesSent: 0,
    totalBytes: 0,
    status: '' as string,
  },
})

export function hex(data: number[] | undefined): string {
  return (data ?? [])
    .map((b) => (b & 0xff).toString(16).padStart(2, '0'))
    .join(' ')
}

export function pushLog(msg: string) {
  const t = new Date().toLocaleTimeString()
  state.log.unshift(`[${t}] ${msg}`)
  if (state.log.length > 500) state.log.pop()
}

function addTrace(type: string, dir: string, f: FrameLike, info = '') {
  const idNum = f.id?.Standard ?? f.id?.Extended ?? 0
  state.trace.unshift({
    id: state.trace.length + 1,
    ts: Date.now(),
    type,
    dir,
    dataHex: hex(f.data),
    info: info || `ID 0x${idNum.toString(16).toUpperCase()}`,
  })
  if (state.trace.length > 2000) state.trace.pop()
}

export function handleEvent(ev: CanEvent) {
  if ('FrameReceived' in ev) addTrace('CAN', 'RX', ev.FrameReceived)
  else if ('FdFrameReceived' in ev)
    addTrace('CAN-FD', 'RX', ev.FdFrameReceived, `BRS=${ev.FdFrameReceived.brs} ESI=${ev.FdFrameReceived.esi}`)
  else if ('FrameSent' in ev) pushLog(`TX 帧 ID=0x${ev.FrameSent.id.toString(16)} len=${ev.FrameSent.len}`)
  else if ('IsoTpReceived' in ev)
    addTrace('ISO-TP', 'RX', { id: { Standard: ev.IsoTpReceived.rx_id }, data: ev.IsoTpReceived.data })
  else if ('UdsRequest' in ev)
    pushLog(`UDS 请求 SID=0x${ev.UdsRequest.service.toString(16)} data=${hex(ev.UdsRequest.payload)}`)
  else if ('UdsResponse' in ev)
    pushLog(`UDS 响应 SID=0x${ev.UdsResponse.service.toString(16)} data=${hex(ev.UdsResponse.payload)}`)
  else if ('UdsNegativeResponse' in ev)
    pushLog(`UDS 负响应 SID=0x${ev.UdsNegativeResponse.service.toString(16)} NRC=0x${ev.UdsNegativeResponse.nrc.toString(16)}`)
  else if ('UdsTimeout' in ev) pushLog(`UDS 超时 SID=0x${ev.UdsTimeout.service.toString(16)}`)
  else if ('BusReady' in ev) pushLog(`总线就绪: ${ev.BusReady.interface}`)
  else if ('BusError' in ev) pushLog(`总线错误: ${ev.BusError.description}`)
  else if ('FlashProgress' in ev) {
    state.flash.blockSeq = ev.FlashProgress.block_seq
    state.flash.totalBlocks = ev.FlashProgress.total_blocks
    state.flash.bytesSent = ev.FlashProgress.bytes_sent
    state.flash.totalBytes = ev.FlashProgress.total_bytes
  } else if ('FlashComplete' in ev) {
    state.flash.active = false
    state.flash.status = `完成: ${ev.FlashComplete.total_bytes} 字节, 耗时 ${ev.FlashComplete.elapsed_ms} ms`
    pushLog(state.flash.status)
  } else if ('FlashError' in ev) {
    state.flash.active = false
    state.flash.status = `失败: ${ev.FlashError.reason}`
    pushLog(state.flash.status)
  }
}
