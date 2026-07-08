# tx_di_file — 统一文件存储插件使用文档

基于 [Apache OpenDAL](https://opendal.apache.org/) 的统一文件存储插件，封装 `FileStorage` trait。

## 用途

- 支持多种对象存储后端：**本地文件系统**（默认 `local` feature）与 **AWS S3 / MinIO 兼容**（可选 `s3` feature）。
- 多后端共存管理：配置文件自动注册的系统后端用 `sys:` 前缀（不可移除），运行时动态添加的自定义后端用 `user:` 前缀（可移除）。
- 支持流式读写（避免大文件占满内存）与便捷的小文件字节上传/下载。

## 启用

`Cargo.toml`：

```toml
tx_di_file = { path = "plugins/tx_di_file" }            # 含默认 local + s3
# 仅本地: tx_di_file = { path = "plugins/tx_di_file", default-features = false, features = ["local"] }
# 仅 S3:  tx_di_file = { path = "plugins/tx_di_file", default-features = false, features = ["s3"] }
```

feature：`local`（默认，基于 `opendal/services-fs`）、`s3`（默认，基于 `opendal/services-s3`）。

## 配置

TOML 节名为 `[file_config]`：

```toml
[file_config]
base_path = "./uploads"
base_url = "http://localhost:8080/files"
max_file_size = 10485760          # 10MB，0 表示不限制
allowed_extensions = ["jpg", "png", "pdf"]

[[file_config.extra_storages]]
name = "s3-images"
backend = "s3"
bucket = "my-bucket"
region = "ap-southeast-1"
endpoint = "http://localhost:9000"
force_path_style = true           # MinIO 通常需要 true
```

| 字段 | 类型 | 默认值 |
|------|------|--------|
| `base_path` | `String` | `"./uploads"` |
| `base_url` | `String` | `""` |
| `max_file_size` | `u64` | `0`（不限制） |
| `allowed_extensions` | `Vec<String>` | `[]`（不限制） |
| `extra_storages` | `Vec<StorageConfig>` | `[]` |

`StorageConfig`：`name`(String)、`backend`(`StorageBackend`: Local/S3/Database)、`base_path`(String, 默认 `"uploads"`)、`base_url`(String)、`s3`(`S3Config`)。

`S3Config`：`bucket`(默认 `""`)、`region`(默认 `"ap-southeast-1"`)、`endpoint`(默认 `""`)、`access_key`(默认 `""`)、`secret_key`(默认 `""`)、`force_path_style`(默认 `false`)。

## 公共组件

| 结构体 | `#[component(...)]` | 说明 |
|--------|----------------------|------|
| `FileConfig` | `conf`, `init`, `init_sort = i32::MIN + 3` | 配置载体 |
| `FilePlugin` | `app_async_init`, `init_sort = i32::MIN + 3` | 存储后端管理门面 |

`FilePlugin` 方法：`get_storage(key)` / `default_storage()`（返回 `sys:local`）/ `add_storage(key, storage)` / `remove_storage(key)`（仅 `user:` 可移除）/ `storage_keys()` / `storage_keys_by_prefix(prefix)` / `sys_storage_keys()` / `user_storage_keys()`。

`FileStorage` trait 核心流式方法：`write_stream` / `read_stream` / `delete` / `exists` / `info` / `list_stream` / `presigned_url`；便捷方法：`upload(data, content_type)`（小文件字节）/ `download() -> Vec<u8>` / `list(prefix)`。`OpendalStorage` 是具体实现结构体（非 DI 组件，手动构造后注册到 `FilePlugin`）。

## 使用方式

```rust
use std::sync::Arc;
use tx_di_core::{BuildContext, Component};
use tx_di_file::{FilePlugin, user_key, storage::{FileStorage, OpendalStorage}};

#[derive(Component)]
pub struct MyService { pub file_plugin: Arc<FilePlugin> }

impl MyService {
    async fn demo(&self) -> tx_error::AppResult<()> {
        let local = self.file_plugin.default_storage().unwrap();
        local.upload("avatars/1.png", b"hello", Some("image/png")).await?;
        let data = local.download("avatars/1.png").await?;

        // 流式上传
        let mut reader = &b"bigdata"[..];
        let path = local.write_stream("big.bin", &mut reader, Some("application/octet-stream")).await?;

        // 获取指定后端
        let s3 = self.file_plugin.get_storage("sys:s3-images").unwrap();

        // 运行时动态添加用户后端
        self.file_plugin.add_storage(
            user_key("my-oss"),
            Arc::new(OpendalStorage::new_s3(&Default::default(), "")?),
        );
        self.file_plugin.remove_storage("user:my-oss")?;
        Ok(())
    }
}

// 启动：app_async_init 自动注册 sys:* 后端
// let app = BuildContext::new::<PathBuf>(Some("config.toml")).build()?.ins_run().await?;
```

## 注意事项

1. **后端 key 前缀规则**：配置自动注册的后端以 `sys:` 为前缀（`sys:local`、`sys:<name>`），`remove_storage` 不可移除，否则返回 `CannotRemoveSystemStorage`；运行时动态添加必须用 `user:` 前缀才可移除。
2. `default_storage()` 固定返回 `sys:local`。
3. **`max_file_size` / `allowed_extensions` 是声明式配置，不强制校验**：插件本身不做大小与扩展名校验（大小限制由 axum `BodySizeLimitLayer`、扩展名白名单由业务层实现）。
4. S3 默认 virtual-host style；MinIO 等通常需 `force_path_style = true`。
5. 未启用 `s3` feature 但配置 `backend = "s3"` 时，`app_async_init` 跳过该后端（仅打 error 日志，不中断启动）。
6. `StorageBackend::Database` 不被 `OpendalStorage` 支持（构造返回 `StorageInitFailed`）。
7. 本地 `presigned_url` 不支持签名，回退到 `base_url + path` 公开 URL（S3 返回真实签名 URL）。
8. `base_url` 仅用于生成对外访问 URL，不影响实际存储位置。
9. `init_sort = i32::MIN + 3`：确保配置先于插件初始化、插件在应用早期完成后端注册。
