// Tauri command 调用封装 + 事件订阅
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { CanConfig, CanEvent, FlashConfigInput, FrameInput } from '../types'

export const api = {
  defaultConfig: () => invoke<CanConfig>('default_config'),
  getConfig: () => invoke<CanConfig | null>('get_config'),
  isConnected: () => invoke<boolean>('is_connected'),
  connect: (config: CanConfig) => invoke<void>('connect', { config }),
  disconnect: () => invoke<void>('disconnect'),
  sendFrame: (frame: FrameInput) => invoke<void>('send_frame', { frame }),
  sendFdFrame: (frame: FrameInput) => invoke<void>('send_fd_frame', { frame }),
  readData: (txId: number, did: number) => invoke<string>('read_data', { txId, did }),
  writeData: (txId: number, did: number, data: number[]) =>
    invoke<void>('write_data', { txId, did, data }),
  sessionControl: (txId: number, session: number) =>
    invoke<void>('session_control', { txId, session }),
  ecuReset: (txId: number, resetType: number) =>
    invoke<void>('ecu_reset', { txId, resetType }),
  testerPresent: (txId: number) => invoke<void>('tester_present', { txId }),
  securityAccess: (txId: number, level: number, keyAlgo: string) =>
    invoke<void>('security_access', { txId, level, keyAlgo }),
  readDtc: (txId: number, statusMask: number) =>
    invoke<string[]>('read_dtc', { txId, statusMask }),
  flash: (firmwarePath: string, config: FlashConfigInput, keyAlgo: string) =>
    invoke<void>('flash', { firmwarePath, config, keyAlgo }),
}

export function onCanEvent(cb: (e: CanEvent) => void): Promise<UnlistenFn> {
  return listen<CanEvent>('can://event', (ev) => cb(ev.payload))
}
