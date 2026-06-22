use admin_proto::{OperateLogResponse, LoginLogResponse};

/// 领域模型 → Proto 响应：操作日志
pub fn operate_log_to_response(log: admin_domain::log::model::aggregate::OperateLog) -> OperateLogResponse {
    OperateLogResponse {
        id: log.id,
        trace_id: log.trace_id,
        user_id: log.user_id,
        user_type: log.user_type,
        log_type: log.log_type,
        sub_type: log.sub_type,
        biz_id: log.biz_id,
        action: log.action,
        success: log.success,
        extra: log.extra,
        request_method: log.request_method,
        request_url: log.request_url,
        user_ip: log.user_ip,
        created_at: Some(log.audit.create_time.to_string()),
    }
}

/// 领域模型 → Proto 响应：登录日志
pub fn login_log_to_response(log: admin_domain::log::model::aggregate::LoginLog) -> LoginLogResponse {
    LoginLogResponse {
        id: log.id,
        user_id: log.user_id,
        user_type: log.user_type,
        username: log.username,
        login_ip: log.login_ip,
        login_type: log.login_type,
        result: log.result,
        msg: log.msg,
    }
}
