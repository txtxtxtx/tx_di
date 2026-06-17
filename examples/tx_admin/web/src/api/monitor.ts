import request from './request'
import type { ApiRes, ServerInfo, OnlineUserListResponse } from '@/types'

export function getServerInfo() {
  return request.get<ApiRes<ServerInfo>>('/api/monitor/server').then(r => r.data)
}

export function getOnlineUsers() {
  return request.get<ApiRes<OnlineUserListResponse>>('/api/monitor/online').then(r => r.data)
}
