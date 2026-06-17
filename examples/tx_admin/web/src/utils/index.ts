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
export function formatTimestamp(ts: number): string {
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

/**
 * 性别映射
 */
export const sexOptions = [
  { label: '未知', value: 0 },
  { label: '男', value: 1 },
  { label: '女', value: 2 },
]

export function sexLabel(val: number): string {
  return sexOptions.find(o => o.value === val)?.label || '未知'
}

/**
 * 用户状态映射
 */
export const userStatusOptions = [
  { label: '正常', value: 0, type: 'success' as const },
  { label: '停用', value: 1, type: 'danger' as const },
  { label: '锁定', value: 2, type: 'warning' as const },
]

export function userStatusLabel(val: number): string {
  return userStatusOptions.find(o => o.value === val)?.label || '未知'
}

export function userStatusType(val: number): string {
  return userStatusOptions.find(o => o.value === val)?.type || 'info'
}

/**
 * 菜单类型映射
 */
export const menuTypeOptions = [
  { label: '目录', value: 0 },
  { label: '菜单', value: 1 },
  { label: '按钮', value: 2 },
]

export function menuTypeLabel(val: number): string {
  return menuTypeOptions.find(o => o.value === val)?.label || '未知'
}

/**
 * 通用状态映射
 */
export const statusOptions = [
  { label: '正常', value: 0, type: 'success' as const },
  { label: '停用', value: 1, type: 'danger' as const },
]

export function statusLabel(val: number): string {
  return statusOptions.find(o => o.value === val)?.label || '未知'
}

/**
 * 权限类型映射
 */
export const permissionTypeOptions = [
  { label: '菜单', value: 0 },
  { label: '按钮', value: 1 },
  { label: 'API', value: 2 },
]

export function permissionTypeLabel(val: number): string {
  return permissionTypeOptions.find(o => o.value === val)?.label || '未知'
}

/**
 * 配置类型映射
 */
export const configTypeOptions = [
  { label: '系统', value: 1 },
  { label: '自定义', value: 2 },
]

export function configTypeLabel(val: number): string {
  return configTypeOptions.find(o => o.value === val)?.label || '未知'
}

/**
 * 可见性映射
 */
export const visibleOptions = [
  { label: '显示', value: 0 },
  { label: '隐藏', value: 1 },
]

export function visibleLabel(val: number): string {
  return visibleOptions.find(o => o.value === val)?.label || '未知'
}

/**
 * 数据范围映射
 */
export const dataScopeOptions = [
  { label: '全部数据', value: 1 },
  { label: '自定义数据', value: 2 },
  { label: '本部门数据', value: 3 },
  { label: '本部门及以下', value: 4 },
  { label: '仅本人数据', value: 5 },
]

export function dataScopeLabel(val: number): string {
  return dataScopeOptions.find(o => o.value === val)?.label || '未知'
}
