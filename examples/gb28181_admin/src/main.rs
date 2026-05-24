//! # GB28181 管理后台
//!
//! 启动 GB28181 服务端 + 管理 HTTP API + Vue3 前端静态服务 + 数据库持久化 + 用户认证。
//!
//! ## 启动步骤
//!
//! ```bash
//! # 1. 先构建前端（首次或代码变更时）
//! cd examples/gb28181_admin/ui && npm install && npm run build
//!
//! # 2. 启动后端
//! cargo run -p gb28181_admin
//! # 打开浏览器访问 http://localhost:8080/admin/
//! ```

mod api;
mod dto;
mod models;

mod api_register;
use tx_di_core::BuildContext;

#[allow(unused_imports)]
use tx_di_axum;
#[allow(unused_imports)]
use tx_di_toasty;
#[allow(unused_imports)]
use tx_di_sa_token;
#[allow(unused_imports)]
use tx_di_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // examples/gb28181_admin/config/config.toml
    // C:\a_me\proj\rust\tx_di\examples\gb28181_admin\config\config.toml
    let app = BuildContext::new(Some(r"C:\a_me\proj\rust\tx_di\examples\gb28181_admin\config\config.toml"))
        .build()?
        .ins_run()
        .await?;


    app.waiting_exit().await;
    Ok(())
}
