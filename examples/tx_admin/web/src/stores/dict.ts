import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getDictDataByType } from '@/api/dict'
import type { DictDataResponse } from '@/types'

const CACHE_PREFIX = 'dict_'

export const useDictStore = defineStore('dict', () => {
  /** 内存缓存：dictType -> DictDataResponse[] */
  const dictMap = ref<Record<string, DictDataResponse[]>>({})

  /**
   * 获取字典数据（懒加载 + localStorage 缓存）
   * 优先级：内存 > localStorage > API
   */
  async function getDictData(dictType: string): Promise<DictDataResponse[]> {
    // 1. 内存有
    if (dictMap.value[dictType]) {
      return dictMap.value[dictType]
    }

    // 2. localStorage 有
    const cached = localStorage.getItem(CACHE_PREFIX + dictType)
    if (cached) {
      try {
        const list = JSON.parse(cached) as DictDataResponse[]
        dictMap.value[dictType] = list
        return list
      } catch {
        localStorage.removeItem(CACHE_PREFIX + dictType)
      }
    }

    // 3. 从 API 获取
    const res = await getDictDataByType(dictType)
    const list = res.data ?? []
    dictMap.value[dictType] = list
    localStorage.setItem(CACHE_PREFIX + dictType, JSON.stringify(list))
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
