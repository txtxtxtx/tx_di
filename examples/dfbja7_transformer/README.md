# dfbja7_transformer

NANO4S设备协议解析与MQTT转发服务

基于 [tx-di-core](../../tx-di-core) 依赖注入框架构建。

## 功能特性

- 解析NANO4S私有协议（支持Nano4SP和GQB200A7U设备）
- 将解析后的数据转换为JSON格式
- 通过MQTT协议转发到指定broker
- 使用 tx-di-core 框架实现依赖注入
- 集成 tx_di_log 日志插件（tracing）
- TOML 配置文件管理

## 项目结构

```
dfbja7_transformer/
├── Cargo.toml
├── config/
│   └── config.toml           # TOML 配置文件
└── src/
    ├── main.rs               # 程序入口（极简）
    ├── config.rs             # AppConfig 配置组件
    ├── mqtt.rs               # MqttClient 组件
    ├── server.rs             # TcpServer 组件
    ├── protocol/             # 协议解析模块
    │   ├── mod.rs
    │   ├── base.rs
    │   ├── nano4sp.rs
    │   └── gqb200a7u.rs
    ├── model/                # 数据模型模块
    │   ├── mod.rs
    │   ├── nano4sp.rs
    │   └── gqb200a7u.rs
    └── util/                 # 工具模块
        ├── mod.rs
        ├── crc32.rs
        ├── ieee754.rs
        └── convert.rs
```

## 架构设计

### 组件依赖关系

```
┌─────────────────────────────────────────────────────────────────┐
│                         main.rs                                 │
│                    BuildContext::new()                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     AppConfig (配置组件)                          │
│                   #[tx_comp(conf, init)]                         │
│                    init_sort: i32::MIN + 1                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   MqttClient (MQTT客户端)                        │
│                      #[tx_comp(init)]                            │
│                    init_sort: 10000                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   ProtocolParser (协议解析器)                     │
│                         #[tx_comp]                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   TcpServer (TCP服务器)                           │
│                      #[tx_comp(init)]                            │
│                    init_sort: i32::MAX                            │
│    async_run_impl: 启动TCP监听                                    │
└─────────────────────────────────────────────────────────────────┘
```

### 初始化顺序

| 组件 | init_sort | 说明 |
|------|-----------|------|
| LogConfig | i32::MIN | 日志配置（tx_di_log 插件） |
| LogPlugins | i32::MIN | 日志初始化（tx_di_log 插件） |
| AppConfig | i32::MIN + 1 | 应用配置 |
| MqttClient | 10000 | MQTT 客户端 |
| ProtocolParser | 10000 | 协议解析器（无状态） |
| TcpServer | i32::MAX | TCP 服务器（最后启动） |

## 配置说明

### 配置文件

编辑 `config/config.toml` 文件：

```toml
# 日志配置
[log_config]
level = "info"
prefix = "dfbja7"
dir = "./logs"
console_output = true
time_format = "local"

# 应用配置
[app_config]
tcp_port = 10080
tcp_timeout_secs = 150
mqtt_broker = "localhost"
mqtt_port = 1883
mqtt_client_id = "dfbja7_transformer"
mqtt_username = ""
mqtt_password = ""
mqtt_topic_prefix = "/device/"
```

## 编译与运行

### 编译

```bash
cargo build --release
```

### 运行

```bash
cargo run --release
```

## 协议说明

### 数据包格式

```
+--------+--------+--------+--------+--------+
| 开始标志 | 长度   | 数据   | RSSI   | CRC    |
+--------+--------+--------+--------+--------+
| 1字节   | 1字节  | 变长   | 1字节  | 2字节  |
+--------+--------+--------+--------+--------+
```

- 开始标志: 0x55 (上行) 或 0xAA (下行)
- 长度: 压缩长度，>128时使用特殊编码
- CRC: CRC32/MPEG-2 校验码

### 支持设备类型

1. **Nano4SP** (模板ID: 04_23_01)
2. **GQB200A7U** (模板ID: 07_1D_00)

## MQTT消息格式

解析后的数据以JSON格式发送到MQTT，主题格式为：`{topic_prefix}{device_model}/{device_code}`

示例消息：

```json
{
  "device_model": "Nano4SP",
  "device_code": "12345678",
  "rssi": "-75dBm",
  "sensors": {
    "sensor1": "100",
    "sensor2": "200",
    "sensor3": "300",
    "sensor4": "4.5"
  },
  "gps": {
    "longitude": "116.397128",
    "latitude": "39.916527"
  },
  "alarm": {
    "levels": [1, 0, 2, 0],
    "level_descriptions": ["通道1一级报警", "通道3二级报警"],
    "special": ["SOS报警"]
  },
  "soc": "85",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## 测试

运行单元测试：

```bash
cargo test
```

## 扩展新设备类型

1. 在 `protocol/` 目录下创建新的设备协议解析器
2. 在 `model/` 目录下创建对应的数据模型
3. 在 `protocol/mod.rs` 中注册新的设备类型
4. 在 `model/mod.rs` 中添加相应的转换逻辑

## 依赖项

- tx-di-core: 依赖注入框架
- tx_di_log: 日志插件
- tokio: 异步运行时
- rumqttc: MQTT客户端
- serde/serde_json: 序列化
- bytes: 字节处理
- tracing: 日志
- anyhow: 错误处理

## 许可证

MIT License
