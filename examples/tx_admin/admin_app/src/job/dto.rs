use admin_proto::{JobResponse, JobLogResponse};
use tx_di_job::{InfrustJob, InfrustJobLog};

// Re-export proto request types directly
pub use admin_proto::{CreateJobRequest, UpdateJobRequest, ListJobsRequest, ListJobLogsRequest, CleanJobLogsRequest};

/// 将 InfrustJob 转换为 proto JobResponse
pub fn job_to_response(job: InfrustJob) -> JobResponse {
    JobResponse {
        id: job.id as u64,
        name: job.name,
        status: job.status as i32,
        handler_name: job.handler_name,
        handler_param: job.handler_param,
        cron_expression: job.cron_expression,
        retry_count: job.retry_count,
        retry_interval: job.retry_interval,
        monitor_timeout: job.monitor_timeout,
    }
}

/// 将 InfrustJobLog 转换为 proto JobLogResponse
pub fn job_log_to_response(log: InfrustJobLog) -> JobLogResponse {
    JobLogResponse {
        id: log.id as u64,
        job_id: log.job_id as u64,
        handler_name: log.handler_name,
        handler_param: log.handler_param,
        execute_index: log.execute_index as i32,
        begin_time: log.begin_time.as_millisecond().to_string(),
        end_time: log.end_time.map(|t| t.as_millisecond().to_string()),
        duration: log.duration,
        status: log.status as i32,
        result: log.result,
    }
}
