// 国际化（中英双语）框架
import { reactive } from 'vue'

export type Lang = 'zh' | 'en'

type Entry = { zh: string; en: string }

const dict: Record<string, Entry> = {
  'tab.trace': { zh: '总线监控', en: 'Trace' },
  'tab.uds': { zh: 'UDS 诊断', en: 'UDS' },
  'tab.flash': { zh: '固件刷写', en: 'Flash' },
  'tab.simecu': { zh: 'ECU 仿真', en: 'Sim ECU' },
  'tab.record': { zh: '录制回放', en: 'Record/Replay' },
  'tab.dbc': { zh: 'DBC 解码', en: 'DBC' },
  'tab.config': { zh: '连接配置', en: 'Config' },
  'tab.xcp': { zh: 'XCP 标定', en: 'XCP' },
  'tab.audit': { zh: '审计报表', en: 'Audit' },
  'tab.project': { zh: '工程管理', en: 'Project' },
  'common.connected': { zh: '已连接', en: 'Connected' },
  'common.disconnected': { zh: '未连接', en: 'Disconnected' },
  'common.refresh': { zh: '刷新', en: 'Refresh' },
  'common.export': { zh: '导出', en: 'Export' },
  'common.clear': { zh: '清空', en: 'Clear' },
  'common.load': { zh: '加载', en: 'Load' },
  'common.save': { zh: '保存', en: 'Save' },
  'title.xcp': { zh: 'XCP on CAN 标定', en: 'XCP on CAN Calibration' },
  'title.audit': { zh: '操作审计与报表', en: 'Audit & Reports' },
  'title.project': { zh: '工程管理（.canproj）', en: 'Project Management' },
  'title.offline': { zh: '离线分析', en: 'Offline Analysis' },
}

export const i18n = reactive({
  lang: (localStorage.getItem('txdi_lang') || 'zh') as Lang,
})

export function t(key: string): string {
  const e = dict[key]
  if (!e) return key
  return e[i18n.lang] ?? e.zh
}

export function setLang(l: Lang) {
  i18n.lang = l
  localStorage.setItem('txdi_lang', l)
}

export function toggleLang() {
  setLang(i18n.lang === 'zh' ? 'en' : 'zh')
}
