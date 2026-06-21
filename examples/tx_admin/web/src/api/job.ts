import request from './request'
import type {
  JobResponse, JobLogResponse,
  CreateJobRequest, UpdateJobRequest,
  ListJobsRequest, ListJobLogsRequest, ChangeJobStatusRequest, CleanJobLogsRequest,
  PageData, ApiRes,
} from '@/types'

// ==================== 任务管理 ====================

/** 任务列表 */
export function listJobs(data: ListJobsRequest) {
  return request.post<ApiRes<PageData<JobResponse>>>('/api/job/list', data).then(r => r.data)
}

/** 任务详情 */
export function getJob(id: string) {
  return request.get<ApiRes<JobResponse>>(`/api/job/${id}`).then(r => r.data)
}

/** 新增任务 */
export function createJob(data: CreateJobRequest) {
  return request.post<ApiRes<JobResponse>>('/api/job', data).then(r => r.data)
}

/** 修改任务 */
export function updateJob(id: string, data: Omit<UpdateJobRequest, 'id'>) {
  return request.put<ApiRes<JobResponse>>(`/api/job/${id}`, { ...data, id }).then(r => r.data)
}

/** 删除任务 */
export function deleteJob(id: string) {
  return request.delete<ApiRes<null>>(`/api/job/${id}`).then(r => r.data)
}

/** 修改任务状态 */
export function changeJobStatus(id: string, status: number) {
  return request.put<ApiRes<JobResponse>>(`/api/job/${id}/status`, { id, status }).then(r => r.data)
}

/** 手动执行任务 */
export function runJob(id: string) {
  return request.post<ApiRes<null>>(`/api/job/${id}/run`).then(r => r.data)
}

// ==================== 执行日志 ====================

/** 日志列表 */
export function listJobLogs(data: ListJobLogsRequest) {
  return request.post<ApiRes<PageData<JobLogResponse>>>('/api/job/log/list', data).then(r => r.data)
}

/** 日志详情 */
export function getJobLog(id: string) {
  return request.get<ApiRes<JobLogResponse>>(`/api/job/log/${id}`).then(r => r.data)
}

/** 清空日志 */
export function cleanJobLogs(jobId?: string) {
  return request.delete<ApiRes<null>>('/api/job/log/clean', { data: { jobId: jobId || null } }).then(r => r.data)
}
