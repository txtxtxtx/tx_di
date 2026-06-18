import request from './request'
import type { ApiRes, PageData, ConfigResponse, CreateConfigRequest, UpdateConfigRequest, ListConfigsRequest } from '@/types'

export function createConfig(data: CreateConfigRequest) {
  return request.post<ApiRes<ConfigResponse>>('/api/config/', data).then(r => r.data)
}

export function getConfig(configId: string) {
  return request.get<ApiRes<ConfigResponse>>(`/api/config/${configId}`).then(r => r.data)
}

export function updateConfig(configId: string, data: UpdateConfigRequest) {
  return request.put<ApiRes<ConfigResponse>>(`/api/config/${configId}`, data).then(r => r.data)
}

export function deleteConfig(configId: string) {
  return request.delete<ApiRes<null>>(`/api/config/${configId}`).then(r => r.data)
}

export function listConfigs(data: ListConfigsRequest) {
  return request.post<ApiRes<PageData<ConfigResponse>>>('/api/config/list', data).then(r => r.data)
}

export function getConfigByKey(key: string) {
  return request.get<ApiRes<ConfigResponse>>(`/api/config/key/${key}`).then(r => r.data)
}
