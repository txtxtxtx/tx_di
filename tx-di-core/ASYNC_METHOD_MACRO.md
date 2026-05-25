# async_method! 宏使用指南

## 概述

`tx_di_core::async_method!` 宏用于简化 `CompInit` trait 中异步方法的实现，避免手动编写冗长的 `impl Future<Output = RIE<()>> + Send` 类型签名。

## 基本用法

### 传统写法（啰嗦）

```rust
impl CompInit for ToastyPlugin {
    fn async_init_impl(ctx: Arc<App>, token: CancellationToken) -> impl Future<Output = RIE<()>> + Send {
        async move {
            let plugin = ctx.inject::<ToastyPlugin>();
            // ... 初始化逻辑
            Ok(())
        }
    }
    
    fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> impl Future<Output = RIE<()>> + Send {
        async move {
            // ... 运行逻辑
            Ok(())
        }
    }
}
```

### 使用宏（简洁）

```rust
impl CompInit for ToastyPlugin {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            let plugin = ctx.inject::<ToastyPlugin>();
            // ... 初始化逻辑
            Ok(())
        }
    );
    
    tx_di_core::async_method!(
        fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            // ... 运行逻辑
            Ok(())
        }
    );
}
```

## 宏展开

上述宏调用会被展开为：

```rust
fn async_init_impl(ctx: Arc<App>, token: CancellationToken) -> impl Future<Output = RIE<()>> + Send {
    async move {
        let plugin = ctx.inject::<ToastyPlugin>();
        // ... 初始化逻辑
        Ok(())
    }
}
```

## 支持的语法

### 1. 基本参数

```rust
tx_di_core::async_method!(
    fn my_async_method(param1: Type1, param2: Type2) -> ReturnType {
        // 函数体
    }
);
```

### 2. 带属性注释

```rust
tx_di_core::async_method!(
    #[allow(unused_variables)]
    fn my_async_method(ctx: Arc<App>) -> RIE<()> {
        // 函数体
    }
);
```

### 3. 可见性修饰符

```rust
tx_di_core::async_method!(
    pub fn my_async_method(ctx: Arc<App>) -> RIE<()> {
        // 函数体
    }
);
```

### 4. 尾部逗号（可选）

```rust
tx_di_core::async_method!(
    fn my_async_method(
        ctx: Arc<App>,
        token: CancellationToken,  // 尾部逗号允许
    ) -> RIE<()> {
        // 函数体
    }
);
```

## 实际示例

### 示例 1：数据库初始化

```rust
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tx_di_core::{App, CompInit, RIE};

struct DatabasePlugin {
    // ... 字段
}

impl CompInit for DatabasePlugin {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let plugin = ctx.inject::<DatabasePlugin>();
            
            // 连接数据库
            tracing::info!("正在连接数据库...");
            // ... 数据库连接逻辑
            
            tracing::info!("数据库连接成功");
            Ok(())
        }
    );
    
    tx_di_core::async_method!(
        fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            let plugin = ctx.inject::<DatabasePlugin>();
            
            // 后台任务
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        tracing::info!("数据库插件收到关闭信号");
                        break;
                    }
                    // ... 其他任务
                }
            }
            
            Ok(())
        }
    );
}
```

### 示例 2：HTTP 服务器

```rust
impl CompInit for HttpServer {
    tx_di_core::async_method!(
        fn async_init_impl(ctx: Arc<App>, _token: CancellationToken) -> RIE<()> {
            let server = ctx.inject::<HttpServer>();
            let config = &server.config;
            
            // 初始化路由
            tracing::info!("初始化 HTTP 服务器，监听端口: {}", config.port);
            
            Ok(())
        }
    );
    
    tx_di_core::async_method!(
        fn async_run_impl(ctx: Arc<App>, token: CancellationToken) -> RIE<()> {
            let server = ctx.inject::<HttpServer>();
            
            // 启动服务器
            let app = server.build_app();
            
            tokio::select! {
                result = axum::serve(server.listener, app) => {
                    result.map_err(|e| anyhow::anyhow!("服务器错误: {}", e))?;
                }
                _ = token.cancelled() => {
                    tracing::info!("HTTP 服务器正在关闭...");
                }
            }
            
            Ok(())
        }
    );
}
```

## 注意事项

1. **必须返回 `RIE<()>`**：宏假设返回值类型为 `RIE<()>`，如果需要其他返回类型，请修改宏定义或使用传统写法。

2. **自动添加 `Send` bound**：宏会自动在返回类型中添加 `+ Send`，确保 Future 可以跨线程传递。

3. **使用 `async move`**：宏内部使用 `async move` 块，因此会移动捕获的变量。如果需要借用，请使用传统写法。

4. **仅适用于 `CompInit` trait**：虽然宏可以用于任何地方，但它是专门为 `CompInit` trait 设计的。

## 与传统写法的对比

| 特性 | 宏写法 | 传统写法 |
|------|--------|----------|
| 代码行数 | 少 | 多 |
| 可读性 | 高 | 中 |
| 灵活性 | 中 | 高 |
| IDE 支持 | 好 | 好 |
| 学习成本 | 低 | 中 |

## 总结

`tx_di_core::async_method!` 宏是一个实用的工具，可以显著简化异步方法的实现代码。推荐在实现 `CompInit` trait 时优先使用此宏，除非有特殊需求需要更灵活的控制。
