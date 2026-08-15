use admin_domain::job::model::aggregate::{Job, JobLog};
use admin_proto::{JobLogResponse, JobResponse};

// Re-export proto request types directly
pub use admin_proto::{
    CleanJobLogsRequest, CreateJobRequest, ListJobLogsRequest, ListJobsRequest, UpdateJobRequest,
};

/// 将领域 `Job` 转换为 proto `JobResponse`
pub fn job_to_response(job: &Job) -> JobResponse {
    JobResponse {
        id: job.id,
        name: job.name.clone(),
        status: job.status as i32,
        handler_name: job.handler_name.clone(),
        handler_param: job.handler_param.clone(),
        cron_expression: job.cron_expression.clone(),
        retry_count: job.retry_count,
        retry_interval: job.retry_interval,
        monitor_timeout: job.monitor_timeout,
    }
}

/// 将领域 `JobLog` 转换为 proto `JobLogResponse`
pub fn job_log_to_response(log: &JobLog) -> JobLogResponse {
    JobLogResponse {
        id: log.id,
        job_id: log.job_id,
        handler_name: log.handler_name.clone(),
        handler_param: log.handler_param.clone(),
        execute_index: log.execute_index as i32,
        begin_time: log.begin_time.as_millisecond().to_string(),
        end_time: log.end_time.map(|t| t.as_millisecond().to_string()),
        duration: log.duration,
        status: log.status as i32,
        result: log.result.clone(),
    }
}
