import request from './request'
import type { ApiRes, PageData, FileResponse, ListFilesRequest, PreviewUrlRes } from '@/types'

/// 流式多文件上传（multipart/form-data）
/// 注意：不要手动设置 Content-Type，浏览器会自动加上 boundary
export function uploadFiles(formData: FormData) {
  return request.post<ApiRes<FileResponse[]>>('/api/v1/file/upload', formData).then(r => r.data)
}

/// 获取文件元数据
export function getFile(fileId: string) {
  return request.get<ApiRes<FileResponse>>(`/api/v1/file/${fileId}`).then(r => r.data)
}

/// 删除文件（物理文件 + DB 软删除）
export function deleteFile(fileId: string) {
  return request.delete<ApiRes<null>>(`/api/v1/file/${fileId}`).then(r => r.data)
}

/// 分页查询文件列表
export function listFiles(data: ListFilesRequest) {
  return request.post<ApiRes<PageData<FileResponse>>>('/api/v1/file/list', data).then(r => r.data)
}

/// 获取文件预览地址
export function getPreviewUrl(fileId: string) {
  return request.get<ApiRes<PreviewUrlRes>>(`/api/v1/file/pre/url/${fileId}`).then(r => r.data)
}
