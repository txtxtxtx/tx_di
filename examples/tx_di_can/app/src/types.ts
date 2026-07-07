// 与后端 tx_di_can 公开 API / CanEvent 对齐的 TS 类型

export type FrameId = { Standard: number } | { Extended: number }

export interface CanFrame {
  id: FrameId
  kind: string
  data: number[]
  timestamp_us: number
}

export interface CanFdFrame {
  id: FrameId
  data: number[]
  brs: boolean
  esi: boolean
  timestamp_us: number
}

export interface CanConfig {
  adapter: string
  interface: string
  bitrate: number
  fd_bitrate: number
  enable_fd: boolean
  rx_queue_size: number
  tx_timeout_ms: number
  isotp_tx_id: number
  isotp_rx_id: number
  isotp_block_size: number
  isotp_st_min_ms: number
  uds_p2_timeout_ms: number
  uds_p2_star_timeout_ms: number
}

export interface FrameInput {
  id: number
  data: number[]
  brs?: boolean
  esi?: boolean
}

export interface FlashConfigInput {
  target_id: number
  security_level?: number
  memory_address?: number
  erase_before_download?: boolean
  default_block_size?: number
}

export interface BusStats {
  frame_count: number
  fd_frame_count: number
  bytes: number
  start_ms: number
  load_permille: number
}

export interface FrameFilter {
  id_min: number | null
  id_max: number | null
  id_mask: number
  id_match: number
}

export interface DescDidInfo {
  id: number
  name: string
  unit: string
}

export interface DescDtcInfo {
  code: number
  text: string
}

export interface DbcSigInfo {
  name: string
  unit: string
  factor: number
  offset: number
  is_signed: boolean
}

export interface DbcMsgInfo {
  id: number
  name: string
  dlc: number
  signals: DbcSigInfo[]
}

export interface DbcValue {
  name: string
  value: number
}

export interface XcpVarInfo {
  name: string
  datatype: string
  address: number
  unit: string
}

export interface XcpA2lInfo {
  module: string
  measurements: XcpVarInfo[]
  characteristics: XcpVarInfo[]
}

export interface XcpValue {
  name: string
  hex: string
  raw: number[]
}

export interface AuditEntryInfo {
  ts_ms: number
  kind: string
  detail: string
  result: string
}

export interface CsvAnalysis {
  total_frames: number
  fd_frames: number
  span_ms: number
  assumed_bitrate: number
  load_permille: number
  avg_interval_ms: number
  top_ids: [number, number][]
}

export interface FlashProjectConfig {
  target_id: number
  security_level: number
  memory_address: number
  erase_before_download: boolean
}

export interface ProjectConfig {
  name: string
  can: CanConfig
  uds_tx_id: number
  recent_dids: number[]
  recent_dtcs: number[]
  flash: FlashProjectConfig
}

export type CanEvent =
  | { BusReady: { interface: string } }
  | { BusError: { description: string } }
  | { FrameReceived: CanFrame }
  | { FdFrameReceived: CanFdFrame }
  | { FrameSent: { id: number; len: number } }
  | { IsoTpReceived: { tx_id: number; rx_id: number; data: number[] } }
  | { UdsRequest: { service: number; payload: number[] } }
  | { UdsResponse: { service: number; payload: number[] } }
  | { UdsNegativeResponse: { service: number; nrc: number } }
  | { UdsTimeout: { service: number } }
  | {
      FlashProgress: {
        block_seq: number
        total_blocks: number
        bytes_sent: number
        total_bytes: number
      }
    }
  | { FlashComplete: { total_bytes: number; elapsed_ms: number } }
  | { FlashError: { reason: string } }
