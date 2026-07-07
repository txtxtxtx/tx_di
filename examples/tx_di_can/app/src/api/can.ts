// Tauri command 调用封装 + 事件订阅
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AuditEntryInfo,
  BusStats,
  CanConfig,
  CanEvent,
  CsvAnalysis,
  DbcMsgInfo,
  DbcValue,
  DescDidInfo,
  DescDtcInfo,
  FlashConfigInput,
  FrameFilter,
  FrameInput,
  ProjectConfig,
  XcpA2lInfo,
  XcpValue,
} from '../types'

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
  getBusStats: () => invoke<BusStats | null>('get_bus_stats'),
  resetStats: () => invoke<void>('reset_stats'),
  setFrameFilter: (filter: FrameFilter | null) =>
    invoke<void>('set_frame_filter', { filter }),
  getFrameFilter: () => invoke<FrameFilter | null>('get_frame_filter'),
  sendIsotp: (txId: number, rxId: number, data: number[]) =>
    invoke<void>('send_isotp', { txId, rxId, data }),
  getDescDids: () => invoke<DescDidInfo[]>('get_desc_dids'),
  getDescDtcs: () => invoke<DescDtcInfo[]>('get_desc_dtcs'),
  simEcuStatus: () => invoke<boolean>('sim_ecu_status'),
  recordCsv: (path: string, durationMs: number) =>
    invoke<number>('record_csv', { path, durationMs }),
  replayCsv: (path: string, speedFactor: number) =>
    invoke<number>('replay_csv', { path, speedFactor }),
  loadDbc: (path: string) => invoke<DbcMsgInfo[]>('load_dbc', { path }),
  decodeDbc: (path: string, canId: number, data: number[]) =>
    invoke<DbcValue[]>('decode_dbc', { path, canId, data }),
  // A. XCP 标定
  xcpParseA2l: (path: string) => invoke<XcpA2lInfo>('xcp_parse_a2l', { path }),
  xcpMeasureAll: (path: string) => invoke<XcpValue[]>('xcp_measure_all', { path }),
  xcpCalibrate: (path: string, name: string, data: number[]) =>
    invoke<void>('xcp_calibrate', { path, name, data }),
  xcpDaqSample: (path: string, name: string) =>
    invoke<XcpValue>('xcp_daq_sample', { path, name }),
  // B. 审计 / 报表
  auditLog: () => invoke<AuditEntryInfo[]>('audit_log'),
  auditClear: () => invoke<void>('audit_clear'),
  exportReport: (path: string, format: string) =>
    invoke<void>('export_report', { path, format }),
  // C. 离线分析
  analyzeCsv: (path: string, bitrate: number) =>
    invoke<CsvAnalysis>('analyze_csv', { path, bitrate }),
  // D. 工程管理
  saveProject: (path: string, cfg: ProjectConfig) =>
    invoke<void>('save_project', { path, cfg }),
  loadProject: (path: string) => invoke<ProjectConfig>('load_project', { path }),
}

export function onCanEvent(cb: (e: CanEvent) => void): Promise<UnlistenFn> {
  return listen<CanEvent>('can://event', (ev) => cb(ev.payload))
}
