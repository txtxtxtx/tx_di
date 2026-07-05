//! 数据模型

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 服务协议类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    Http,
    Grpc,
}

/// 服务端点（一个协议一个地址）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub protocol: Protocol,
    pub ip: String,
    pub port: u16,
    pub metadata: HashMap<String, String>,
}

/// 服务实例（包含多个协议的端点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub service_name: String,
    pub instance_id: String,
    pub endpoints: Vec<ServiceEndpoint>,
    pub healthy: bool,
    pub metadata: HashMap<String, String>,
}
