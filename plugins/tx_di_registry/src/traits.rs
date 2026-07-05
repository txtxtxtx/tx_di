//! 核心 Trait 定义

use tx_error::AppResult;

use crate::model::{ServiceEndpoint, ServiceInstance};

/// 服务注册与发现 trait
#[async_trait::async_trait]
pub trait ServiceRegistry: Send + Sync + 'static {
    /// 注册服务实例（包含全部端点）
    async fn register(&self, instance: &ServiceInstance) -> AppResult<()>;

    /// 更新实例（在线修改端点/元数据）
    async fn update(&self, instance: &ServiceInstance) -> AppResult<()>;

    /// 注销服务实例
    async fn deregister(&self, instance_id: &str) -> AppResult<()>;

    /// 发现服务（按名称获取实例列表）
    async fn discover(&self, service_name: &str) -> AppResult<Vec<ServiceInstance>>;

    /// 监听服务变更
    async fn subscribe(
        &self,
        service_name: &str,
        callback: Box<dyn Fn(Vec<ServiceInstance>) + Send + Sync>,
    );
}

/// 配置中心 trait
#[async_trait::async_trait]
pub trait ConfigCenter: Send + Sync + 'static {
    /// 获取配置（返回原始 JSON 字符串）
    async fn get_config(
        &self,
        data_id: &str,
        group: &str,
    ) -> AppResult<Option<String>>;

    /// 发布/更新配置
    async fn publish_config(
        &self,
        data_id: &str,
        group: &str,
        content: &str,
    ) -> AppResult<()>;

    /// 删除配置
    async fn remove_config(&self, data_id: &str, group: &str) -> AppResult<()>;

    /// 监听配置变更（data_id+group 变更时回调）
    async fn listen_config(
        &self,
        data_id: &str,
        group: &str,
        callback: Box<dyn Fn(String) + Send + Sync>,
    );
}

/// 端点提供者 trait — HTTP/gRPC 插件实现此 trait 来声明自己的端点
pub trait EndpointProvider: Send + Sync {
    /// 返回当前服务提供的所有端点
    fn get_endpoints(&self) -> Vec<ServiceEndpoint>;
}
