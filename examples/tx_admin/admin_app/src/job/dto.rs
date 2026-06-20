use admin_domain::job::model::aggregate::{Job, JobLog};
use admin_proto::{JobResponse, JobLogResponse};

// Re-export proto request types directly
pub use admin_proto::{CreateJobRequest, UpdateJobRequest, ListJobsRequest, ListJobLogsRequest, CleanJobLogsRequest};

/// 将领域层的 Job 聚合根转换为 proto 的 JobResponse
pub fn job_to_response(job: Job) -> JobResponse {
    JobResponse {
        id: job.id,
        name: job.name,
        status: job.status,
        handler_name: job.handler_name,
        handler_param: job.handler_param,
        cron_expression: job.cron_expression,
        retry_count: job.retry_count,
        retry_interval: job.retry_interval,
        monitor_timeout: job.monitor_timeout,
    }
}

/// 将领域层的 JobLog 聚合根转换为 proto 的 JobLogResponse
pub fn job_log_to_response(log: JobLog) -> JobLogResponse {
    JobLogResponse {
        id: log.id,
        job_id: log.job_id,
        handler_name: log.handler_name,
        handler_param: log.handler_param,
        execute_index: log.execute_index,
        begin_time: log.begin_time,
        end_time: log.end_time,
        duration: log.duration,
        status: log.status,
        result: log.result,
    }
}
