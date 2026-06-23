//! 系统工具 gRPC 服务实现（示例 mock 数据）

use tonic::{Request, Response, Status};

use admin_proto::admin::tool::tool_service_server::ToolService;
use admin_proto::admin::tool::CacheStatsResponse;

use super::auth_interceptor::{self, get_login_id};

#[derive(Debug, Clone, Default)]
pub struct ToolGrpcService;

#[tonic::async_trait]
impl ToolService for ToolGrpcService {
    async fn get_cache_stats(
        &self,
        request: Request<()>,
    ) -> Result<Response<CacheStatsResponse>, Status> {
        let login_id = get_login_id(&request)?;
        auth_interceptor::ensure_grpc_permission(&login_id, "system:view").await?;

        // TODO: 当前返回硬编码的 mock 数据，应替换为真实的缓存统计
        Ok(Response::new(CacheStatsResponse {
            total_keys: 1024,
            used_memory: 67_108_864, // 64 MB
            hit_count: 8500,
            miss_count: 1500,
            hit_rate: 85.0,
        }))
    }
}
