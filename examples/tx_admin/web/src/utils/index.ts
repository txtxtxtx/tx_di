import type { DictDataResponse } from '@/types'

/**
 * 格式化字节大小
 */
export function formatBytes(bytes: number, decimals = 2): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i]
}

/**
 * 格式化时间戳 (毫秒) 为本地字符串
 */
export function formatTimestamp(ts: number | string): string {
  if (!ts) return '-'
  return new Date(ts).toLocaleString('zh-CN')
}

/**
 * 格式化日期字符串
 */
export function formatDate(dateStr: string | null | undefined): string {
  if (!dateStr) return '-'
  return dateStr
}

// ==================== 字典数据通用工具 ====================

/**
 * 将字典数据列表转为 el-select/el-radio 选项格式
 * value 为纯数字字符串时自动转为 number，避免后端 i32 反序列化失败
 */
export function dictToOptions(list: DictDataResponse[]) {
  return list.map(d => ({
    label: d.label,
    value: /^\d+$/.test(d.value) ? Number(d.value) : d.value,
    colorType: d.colorType || '',
  }))
}

/**
 * 根据 value 查找 label
 */
export function dictLabel(list: DictDataResponse[], value: string | number): string {
  return list.find(d => d.value === String(value))?.label || String(value)
}

/**
 * 根据 value 查找 colorType（用于 el-tag 的 type 属性）
 */
export function dictColorType(list: DictDataResponse[], value: string | number): string {
  return list.find(d => d.value === String(value))?.colorType || ''
}

// ==================== 后端 i64/u64 序列化为 string 的转换工具 ====================

/**
 * 将后端返回的分页数据中的 string 字段转为 number
 * 后端 i64/u64 通过 serde_with::DisplayFromStr 序列化为 JSON string，
 * 前端 page/size/total 需要 number 类型用于分页组件
 */
export function toPageData<T>(raw: { list?: T[]; page?: string | number; size?: string | number; total?: string | number }): { list: T[]; page: number; size: number; total: number } {
  return {
    list: (raw.list ?? []) as T[],
    page: Number(raw.page) || 1,
    size: Number(raw.size) || 10,
    total: Number(raw.total) || 0,
  }
}

/**
 * 将后端返回的时间戳字段从 string 转为 number
 * 用于 UserResponse、OperateLogResponse 等包含时间戳的类型
 */
export function toTimestamp(val: string | number): number {
  return Number(val) || 0
}
