# ============================================================
# tx_admin 生产镜像（多阶段构建）
# ============================================================
# 用法（在仓库根目录执行）：
#   docker build -f examples/tx_admin/Dockerfile -t tx-admin .
#
# 运行时配置通过环境变量注入：
#   CONFIG_PATH          本地配置文件路径（容器内默认 /app/config/config.toml）
#   APP_HOME             数据/日志/上传等相对路径锚点（默认 /app/data）
#   SERVICE_IP           注册到 Nacos 的实例 IP
#   GRPC_PORT            gRPC 监听端口（覆盖 [grpc_config].port）
#   ADMIN_INIT_PASSWORD  首次建库时的管理员密码
#   REDIS_URL            会话/分布式锁 Redis 连接串（启用 redis feature 时）
# ============================================================

# ─── 阶段 1：构建前端 ─────────────────────────────────────────
FROM node:22-alpine AS web-builder
WORKDIR /src/web
COPY examples/tx_admin/web/package*.json ./
RUN npm ci --registry=https://registry.npmmirror.com
COPY examples/tx_admin/web/ ./
# 生产环境子路径为 /web/（与 config.toml spa_apps 一致）
ENV BASE_URL=/web/
RUN npm run build

# ─── 阶段 2：构建后端 ─────────────────────────────────────────
FROM rust:1.97-slim AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build -p admin_api --release

# ─── 阶段 3：运行时镜像 ───────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home --shell /usr/sbin/nologin appuser

WORKDIR /app
# 后端二进制
COPY --from=builder /src/target/release/admin_api /app/admin_api
# 本地配置文件（可被挂载覆盖）
COPY examples/tx_admin/config/config.toml /app/config/config.toml
# 前端构建产物：路径必须与 config.toml spa_apps 在 APP_HOME 锚定后的结果一致。
# config.toml 中 spa_apps["/web"] = "examples/tx_admin/web/dist"，
# APP_HOME=/app/data 时 resolve_data_path 解析为 /app/data/examples/tx_admin/web/dist，
# 因此前端必须 COPY 到该目录，否则 /web 前端不可访问。
COPY --from=web-builder /src/web/dist /app/data/examples/tx_admin/web/dist

# SQLite 默认库路径（config.toml database_url 锚定后）对应的父目录：
# sqlite:examples/tx_admin/data/tx_admin.db → /app/data/examples/tx_admin/data/tx_admin.db
RUN mkdir -p /app/data/examples/tx_admin/data /app/logs /app/uploads \
    && chown -R appuser:appuser /app

USER appuser
ENV CONFIG_PATH=/app/config/config.toml \
    APP_HOME=/app/data \
    RUST_LOG=info

EXPOSE 8888 50051
# /health/live 探活：进程存活即 200
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8888/health/live || exit 1

ENTRYPOINT ["/app/admin_api"]
