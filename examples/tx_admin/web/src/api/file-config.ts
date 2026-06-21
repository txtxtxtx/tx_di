import request from './request'
import type { FileConfigResponse, CreateFileConfigRequest, UpdateFileConfigRequest, ApiRes } from '@/types'

/** 获取文件配置列表 */
export function listFileConfigs() {
  return request.get<ApiRes<FileConfigResponse[]>>('/api/file/config/list').then(r => r.data)
}

/** 获取文件配置详情 */
export function getFileConfig(id: number) {
  return request.get<ApiRes<FileConfigResponse>>(`/api/file/config/${id}`).then(r => r.data)
}

/** 新增文件配置 */
export function createFileConfig(data: CreateFileConfigRequest) {
  return request.post<ApiRes<FileConfigResponse>>('/api/file/config', data).then(r => r.data)
}

/** 修改文件配置 */
export function updateFileConfig(id: number, data: UpdateFileConfigRequest) {
  return request.put<ApiRes<FileConfigResponse>>(`/api/file/config/${id}`, data).then(r => r.data)
}

/** 删除文件配置 */
export function deleteFileConfig(id: number) {
  return request.delete<ApiRes<null>>(`/api/file/config/${id}`).then(r => r.data)
}

/** 设为主配置 */
export function setMasterFileConfig(id: number) {
  return request.put<ApiRes<FileConfigResponse>>(`/api/file/config/${id}/master`).then(r => r.data)
}
