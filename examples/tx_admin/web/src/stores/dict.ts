import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getDictDataByType } from '@/api/dict'
import type { DictDataResponse } from '@/types'

const CACHE_PREFIX = 'dict_'
// 字典缓存版本号：字典数据结构或业务映射变更时 +1，旧版本缓存会自动失效
const CACHE_VERSION = 'v1'
// 字典缓存有效期（毫秒）：7 天
const CACHE_TTL_MS = 7 * 24 * 60 * 60 * 1000

export const useDictStore = defineStore('dict', () => {
  /** 内存缓存：dictType -> DictDataResponse[] */
  const dictMap = ref<Record<string, DictDataResponse[]>>({})

  /**
   * 获取字典数据（懒加载 + localStorage 缓存）
   * 优先级：内存 > localStorage（未过期且版本一致） > API
   *
   * 缓存带版本号与有效期：
   * - 版本号与当前代码 `CACHE_VERSION` 不一致时缓存失效，强制重新拉取
   *   （解决历史遗留的 value/label 反向等旧缓存导致页面显示错误的问题）
   * - 超过 `CACHE_TTL_MS`（7 天）自动过期，重新拉取最新数据
   */
  async function getDictData(dictType: string): Promise<DictDataResponse[]> {
    // 1. 内存有
    if (dictMap.value[dictType]) {
      return dictMap.value[dictType]
    }

    // 2. localStorage 有（校验版本号与有效期）
    const cacheKey = CACHE_PREFIX + dictType
    const cached = localStorage.getItem(cacheKey)
    if (cached) {
      try {
        const parsed = JSON.parse(cached) as { version?: string; savedAt?: number; data?: DictDataResponse[] }
        const versionOk = !parsed.version || parsed.version === CACHE_VERSION
        const fresh = !parsed.savedAt || Date.now() - parsed.savedAt < CACHE_TTL_MS
        if (versionOk && fresh && Array.isArray(parsed.data)) {
          dictMap.value[dictType] = parsed.data
          return parsed.data
        }
      } catch {
        // 缓存损坏，忽略后重新拉取
      }
      localStorage.removeItem(cacheKey)
    }

    // 3. 从 API 获取
    const res = await getDictDataByType(dictType)
    const list = res.data ?? []
    dictMap.value[dictType] = list
    // 写入带版本号与时间戳的缓存
    const cachePayload = { version: CACHE_VERSION, savedAt: Date.now(), data: list }
    localStorage.setItem(CACHE_PREFIX + dictType, JSON.stringify(cachePayload))
    return list
  }

  /**
   * 获取字典选项列表，适配 el-select / el-tag
   * 返回 [{ label, value, colorType }]
   */
  async function getDictOptions(dictType: string) {
    const list = await getDictData(dictType)
    return list.map(d => ({
      label: d.label,
      value: d.value,
      colorType: d.colorType || '',
    }))
  }

  /**
   * 根据 value 获取 label
   */
  async function getDictLabel(dictType: string, value: string | number): Promise<string> {
    const list = await getDictData(dictType)
    return list.find(d => d.value === String(value))?.label || String(value)
  }

  /**
   * 强制刷新某个字典类型（字典管理页面编辑后调用）
   */
  async function refreshDict(dictType: string): Promise<DictDataResponse[]> {
    localStorage.removeItem(CACHE_PREFIX + dictType)
    delete dictMap.value[dictType]
    return getDictData(dictType)
  }

  /**
   * 清除所有缓存
   */
  function clearCache() {
    dictMap.value = {}
    const keys = Object.keys(localStorage).filter(k => k.startsWith(CACHE_PREFIX))
    keys.forEach(k => localStorage.removeItem(k))
  }

  return { dictMap, getDictData, getDictOptions, getDictLabel, refreshDict, clearCache }
})
