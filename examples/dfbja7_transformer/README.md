# dfbja7_transformer

NANO4S设备协议解析与MQTT转发服务

## 功能特性

- 解析NANO4S私有协议（支持Nano4SP和GQB200A7U设备）
- 将解析后的数据转换为JSON格式
- 通过MQTT协议转发到指定broker
- 支持环境变量配置
- 可扩展的设备类型支持

## 项目结构

```
src/
├── main.rs                 # 程序入口
├── config.rs               # 配置模块
├── error.rs                # 错误定义
├── protocol/               # 协议解析模块
│   ├── mod.rs
│   ├── decoder.rs          # 协议解码器
│   ├── base.rs             # 基础协议结构
│   ├── nano4sp.rs          # Nano4SP设备协议
│   └── gqb200a7u.rs        # GQB200A7U设备协议
├── model/                  # 数据模型模块
│   ├── mod.rs
│   ├── nano4sp.rs          # Nano4SP数据模型
│   └── gqb200a7u.rs        # GQB200A7U数据模型
├── util/                   # 工具模块
│   ├── mod.rs
│   ├── crc32.rs            # CRC32计算
│   ├── ieee754.rs          # IEEE754浮点数转换
│   └── convert.rs          # 转换工具
├── server/                 # TCP服务器模块
│   ├── mod.rs
│   └── tcp.rs              # TCP服务器实现
└── mqtt/                   # MQTT客户端模块
    ├── mod.rs
    └── client.rs           # MQTT客户端实现
```

## 配置说明

### 环境变量

复制 `.env.example` 为 `.env` 文件，然后根据实际情况修改配置：

```bash
cp .env.example .env
```

配置项说明：

- `TCP_PORT`: TCP服务器监听端口（默认: 10080）
- `MQTT_BROKER`: MQTT broker地址（默认: localhost）
- `MQTT_PORT`: MQTT broker端口（默认: 1883）
- `MQTT_CLIENT_ID`: MQTT客户端ID
- `MQTT_USERNAME`: MQTT用户名（可选）
- `MQTT_PASSWORD`: MQTT密码（可选）
- `MQTT_TOPIC`: MQTT主题前缀（默认: /device/）
- `RUST_LOG`: 日志级别（默认: info）

## 编译与运行

### 编译

```bash
cargo build --release
```

### 运行

```bash
cargo run --release
```

或者直接运行编译后的二进制文件：

```bash
./target/release/dfbja7_transformer
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
   - 4路传感器数据
   - GPS经纬度
   - 报警信息
   - 电量信息

2. **GQB200A7U** (模板ID: 07_1D_00)
   - 4路传感器数据
   - GPS经纬度
   - 报警信息

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

- tokio: 异步运行时
- rumqttc: MQTT客户端
- serde/serde_json: 序列化
- bytes: 字节处理
- tracing: 日志
- thiserror/anyhow: 错误处理

## 许可证

MIT License