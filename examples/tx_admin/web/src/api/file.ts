import request from './request'
import type { ApiRes, PageData, FileResponse, UploadFileRequest, ListFilesRequest } from '@/types'

export function uploadFile(data: UploadFileRequest) {
  return request.post<ApiRes<FileResponse>>('/api/file/', data).then(r => r.data)
}

export function getFile(fileId: number) {
  return request.get<ApiRes<FileResponse>>(`/api/file/${fileId}`).then(r => r.data)
}

export function deleteFile(fileId: number) {
  return request.delete<ApiRes<null>>(`/api/file/${fileId}`).then(r => r.data)
}

export function downloadFile(fileId: number) {
  return request.get<ApiRes<{ url: string; filename: string; size: number; contentType: string }>>(`/api/file/${fileId}/download`).then(r => r.data)
}

export function listFiles(data: ListFilesRequest) {
  return request.post<ApiRes<PageData<FileResponse>>>('/api/file/list', data).then(r => r.data)
}
