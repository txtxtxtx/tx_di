import request from './request'
import type { ApiRes, PageData, OperateLogResponse, LoginLogResponse, ListOperateLogsRequest, ListLoginLogsRequest, DeleteLogsRequest } from '@/types'

// 操作日志
export function listOperateLogs(data: ListOperateLogsRequest) {
  return request.post<ApiRes<PageData<OperateLogResponse>>>('/api/v1/log/operate/list', data).then(r => r.data)
}

export function deleteOperateLogs(data: DeleteLogsRequest) {
  return request.post<ApiRes<null>>('/api/v1/log/operate/delete', data).then(r => r.data)
}

export function cleanOperateLogs() {
  return request.delete<ApiRes<null>>('/api/v1/log/operate/clean').then(r => r.data)
}

// 登录日志
export function listLoginLogs(data: ListLoginLogsRequest) {
  return request.post<ApiRes<PageData<LoginLogResponse>>>('/api/v1/log/login/list', data).then(r => r.data)
}

export function deleteLoginLogs(data: DeleteLogsRequest) {
  return request.post<ApiRes<null>>('/api/v1/log/login/delete', data).then(r => r.data)
}

export function cleanLoginLogs() {
  return request.delete<ApiRes<null>>('/api/v1/log/login/clean').then(r => r.data)
}
