import request from './request'
import type { ApiRes, PageData, FileResponse, ListFilesRequest } from '@/types'

/// 流式多文件上传（multipart/form-data）
export function uploadFiles(formData: FormData) {
  return request.post<ApiRes<FileResponse[]>>('/api/file/upload', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  }).then(r => r.data)
}

/// 获取文件元数据
export function getFile(fileId: string) {
  return request.get<ApiRes<FileResponse>>(`/api/file/${fileId}`).then(r => r.data)
}

/// 删除文件（物理文件 + DB 软删除）
export function deleteFile(fileId: string) {
  return request.delete<ApiRes<null>>(`/api/file/${fileId}`).then(r => r.data)
}

/// 分页查询文件列表
export function listFiles(data: ListFilesRequest) {
  return request.post<ApiRes<PageData<FileResponse>>>('/api/file/list', data).then(r => r.data)
}
