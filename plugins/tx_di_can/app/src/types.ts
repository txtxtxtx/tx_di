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
