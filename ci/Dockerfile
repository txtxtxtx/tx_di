# ============================================================
# CI 构建镜像：在 rust:1.97-slim 基础上预装编译 admin_api 所需系统依赖
# ------------------------------------------------------------
# admin_api（gRPC/redis/postgres/reqwest 等）编译时会链接 openssl-sys，
# 需要 pkg-config 与 libssl-dev；同时安装 git（cargo 拉取依赖）与
# ca-certificates。预装到镜像可避免每次流水线重复 apt install，
# 且镜像会被平台自动缓存，仅当本 Dockerfile 变化时才重建。
# ============================================================
FROM rust:1.97-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
        git \
    && rm -rf /var/lib/apt/lists/*
