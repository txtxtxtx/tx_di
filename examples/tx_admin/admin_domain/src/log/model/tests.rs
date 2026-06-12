// ============================================================
// UNIT TESTS: Log 聚合根
// Coverage: OperateLog::create/with_request; LoginLog::create
// ============================================================

#[cfg(test)]
mod log_tests {
    use crate::shared::model::AggregateRoot;
    use crate::log::model::aggregate::{OperateLog, LoginLog};
    use crate::shared::model::DomainEvent;
    use pretty_assertions::assert_eq;

    // ── OperateLog ────────────────────────────────────

    fn make_operate_log() -> OperateLog {
        OperateLog::create(
            1, "trace_001".into(), 42, 1, "user".into(), "create".into(),
            100, "created".into(), 1, "{}".into(),
        )
    }

    #[test]
    fn test_create_operate_log_sets_fields() {
        let log = make_operate_log();
        assert_eq!(log.id, 1);
        assert_eq!(log.trace_id, "trace_001");
        assert_eq!(log.user_id, 42);
        assert_eq!(log.user_type, 1);
        assert_eq!(log.log_type, "user");
        assert_eq!(log.sub_type, "create");
        assert_eq!(log.biz_id, 100);
        assert_eq!(log.action, "created");
        assert_eq!(log.success, 1);
    }

    #[test]
    fn test_create_operate_log_defaults_request_fields() {
        let log = make_operate_log();
        assert!(log.request_method.is_none());
        assert!(log.request_url.is_none());
        assert!(log.user_ip.is_none());
        assert!(log.user_agent.is_none());
    }

    #[test]
    fn test_operate_log_with_request() {
        let log = make_operate_log()
            .with_request(Some("POST".into()), Some("/api/user".into()),
                Some("192.168.1.1".into()), Some("Mozilla".into()));

        assert_eq!(log.request_method.as_deref(), Some("POST"));
        assert_eq!(log.request_url.as_deref(), Some("/api/user"));
        assert_eq!(log.user_ip.as_deref(), Some("192.168.1.1"));
        assert_eq!(log.user_agent.as_deref(), Some("Mozilla"));
    }

    #[test]
    fn test_create_operate_log_raises_event() {
        let log = make_operate_log();
        assert_eq!(log.events().len(), 1);
        assert!(matches!(log.events()[0], DomainEvent::OperateLogCreated { log_id: 1 }));
    }

    // ── LoginLog ──────────────────────────────────────

    fn make_login_log() -> LoginLog {
        LoginLog::create(1, 42, 1, "admin".into(), "192.168.1.1".into(), "password".into(), 1)
    }

    #[test]
    fn test_create_login_log_sets_fields() {
        let log = make_login_log();
        assert_eq!(log.id, 1);
        assert_eq!(log.user_id, 42);
        assert_eq!(log.user_type, 1);
        assert_eq!(log.username, "admin");
        assert_eq!(log.login_ip, "192.168.1.1");
        assert_eq!(log.login_type, "password");
        assert_eq!(log.result, 1);
    }

    #[test]
    fn test_create_login_log_has_login_time() {
        let log = make_login_log();
        assert!(log.login_time.as_millisecond() > 0);
    }

    #[test]
    fn test_create_login_log_defaults_optional_fields() {
        let log = make_login_log();
        assert!(log.login_location.is_none());
        assert!(log.browser.is_none());
        assert!(log.os.is_none());
        assert!(log.msg.is_none());
    }

    #[test]
    fn test_create_login_log_raises_event() {
        let log = LoginLog::create(2, 1, 0, "user".into(), "10.0.0.1".into(), "token".into(), 0);
        assert_eq!(log.events().len(), 1);
        assert!(matches!(log.events()[0], DomainEvent::LoginLogCreated { log_id: 2 }));
    }
}
