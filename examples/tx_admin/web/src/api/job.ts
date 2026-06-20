import request from './request'
import type {
  JobResponse, JobLogResponse,
  CreateJobRequest, UpdateJobRequest,
  ListJobsRequest, ListJobLogsRequest,
} from '@/types'
import type { PageData } from '@/utils'

// ==================== 任务管理 ====================

/** 任务列表 */
export function listJobs(data: ListJobsRequest) {
  return request.post<any, PageData<JobResponse>>('/job/list', data)
}

/** 任务详情 */
export function getJob(id: string) {
  return request.get<any, JobResponse>(`/job/${id}`)
}

/** 新增任务 */
export function createJob(data: CreateJobRequest) {
  return request.post<any, JobResponse>('/job', data)
}

/** 修改任务 */
export function updateJob(id: string, data: UpdateJobRequest) {
  return request.put<any, JobResponse>(`/job/${id}`, data)
}

/** 删除任务 */
export function deleteJob(id: string) {
  return request.delete<any, void>(`/job/${id}`)
}

/** 修改任务状态 */
export function changeJobStatus(id: string, status: number) {
  return request.put<any, JobResponse>(`/job/${id}/status`, { status })
}

/** 手动执行任务 */
export function runJob(id: string) {
  return request.post<any, void>(`/job/${id}/run`)
}

// ==================== 执行日志 ====================

/** 日志列表 */
export function listJobLogs(data: ListJobLogsRequest) {
  return request.post<any, PageData<JobLogResponse>>('/job/log/list', data)
}

/** 日志详情 */
export function getJobLog(id: string) {
  return request.get<any, JobLogResponse>(`/job/log/${id}`)
}

/** 清空日志 */
export function cleanJobLogs(jobId?: string) {
  return request.delete<any, void>('/job/log/clean', { params: { jobId } })
}
