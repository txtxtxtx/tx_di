import request from './request'
import type { ApiRes, ServerInfo, OnlineUserListResponse } from '@/types'

export function getServerInfo(all?: boolean) {
  return request.get<ApiRes<ServerInfo>>('/api/v1/monitor/server', { params: all ? { all: true } : {} }).then(r => r.data)
}

export function getOnlineUsers() {
  return request.get<ApiRes<OnlineUserListResponse>>('/api/v1/monitor/online').then(r => r.data)
}
