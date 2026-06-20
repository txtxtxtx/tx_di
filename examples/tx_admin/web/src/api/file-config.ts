import request from './request'
import type { FileConfigResponse, CreateFileConfigRequest, UpdateFileConfigRequest } from '@/types'

/** 获取文件配置列表 */
export function listFileConfigs() {
  return request.get<any, FileConfigResponse[]>('/file/config/list')
}

/** 获取文件配置详情 */
export function getFileConfig(id: number) {
  return request.get<any, FileConfigResponse>(`/file/config/${id}`)
}

/** 新增文件配置 */
export function createFileConfig(data: CreateFileConfigRequest) {
  return request.post<any, FileConfigResponse>('/file/config', data)
}

/** 修改文件配置 */
export function updateFileConfig(id: number, data: UpdateFileConfigRequest) {
  return request.put<any, FileConfigResponse>(`/file/config/${id}`, data)
}

/** 删除文件配置 */
export function deleteFileConfig(id: number) {
  return request.delete<any, void>(`/file/config/${id}`)
}

/** 设为主配置 */
export function setMasterFileConfig(id: number) {
  return request.put<any, FileConfigResponse>(`/file/config/${id}/master`)
}
