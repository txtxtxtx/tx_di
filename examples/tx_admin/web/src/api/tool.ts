import request from './request'
import type { ApiRes, CacheStatsResponse } from '@/types'

export function getCacheStats() {
  return request.get<ApiRes<CacheStatsResponse>>('/api/v1/tool/cache/stats').then(r => r.data)
}
