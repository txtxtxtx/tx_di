//! 认证相关 API（登录/登出/用户管理）
//!
//! 使用 toasty 0.6 ORM + sa_token-rust 认证
//!
//! # State 策略
//!
//! 所有 handler 使用 `State<Db>`。
//! SaToken 认证通过 `SaTokenLayer` 完成（注入 extensions），
//! handler 通过 `LoginIdExtractor` 从 extensions 提取 login_id，
//! 无需将 `SaTokenState` 放入 axum State。

use axum::{
    extract::{Json as ExtJson, Path, State},
};
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use tx_di_sa_token::{StpUtil, LoginIdExtractor};
use tx_di_axum::R;
use toasty::Db;
use crate::models::User;

// ============ 请求/响应 DTO ============

/// 登录请求体
#[derive(Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginRes {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: u64,
    pub username: String,
    pub nickname: String,
    /// 第一个角色（兼容前端）
    pub role: String,
}

impl From<crate::models::User> for UserInfo {
    fn from(u: crate::models::User) -> Self {
        Self {
            id: u.id,
            username: u.username.clone(),
            nickname: u.nickname.clone(),
            role: u.roles.first().cloned().unwrap_or_else(|| "user".to_string()),
        }
    }
}

/// 创建用户请求体
#[derive(Deserialize)]
pub struct CreateUserReq {
    pub username: String,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_user_roles")]
    pub roles: Vec<String>,
}
fn default_user_roles() -> Vec<String> { vec!["user".to_string()] }

/// 更新用户请求体
#[derive(Deserialize)]
pub struct UpdateUserReq {
    pub nickname: Option<String>,
    pub roles: Option<Vec<String>>,
    pub status: Option<i32>,
}

// ============ Handler 函数 ============

/// POST /api/v1/auth/login — 用户登录
///
/// 返回 `R<LoginRes>`（非 generic impl IntoResponse），确保 R<T> 类型正确推断
pub async fn login(
    State(mut db): State<Db>,
    ExtJson(req): ExtJson<LoginReq>,
) -> R<LoginRes> {
    // 查询用户 — toasty 0.6 正确 API
    let user = match crate::models::User::filter_by_username(req.username.clone())
        .first()
        .exec(&mut db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return R::error(401, "用户名或密码错误".into()),
        Err(e) => return R::error(500, format!("数据库查询失败: {}", e)),
    };

    // 验证密码（password_hash 是 String，不是 Option）
    if !verify(&req.password, &user.password_hash).unwrap_or(false) {
        return R::error(401, "用户名或密码错误".to_string());
    }

    // 检查账号状态
    if user.status != 1 {
        return R::error(403, "账号已被禁用".to_string());
    }

    // 通过 StpUtil 登录
    let token = match StpUtil::login(user.id.to_string()).await {
        Ok(t) => t,
        Err(e) => return R::error(500, format!("登录处理失败: {}", e)),
    };

    R::ok(LoginRes {
        token: token.as_str().to_string(),
        user: UserInfo::from(user),
    })
}

/// POST /api/v1/auth/logout — 用户登出
pub async fn logout() -> R<String> {
    match StpUtil::logout_current().await {
        Ok(_) => R::ok("已登出".to_string()),
        Err(e) => R::fail(format!("登出失败: {}", e)),
    }
}

/// GET /api/v1/auth/info — 获取当前登录用户信息
pub async fn get_info(
    State(mut db): State<Db>,
    LoginIdExtractor(login_id): LoginIdExtractor,
) -> R<UserInfo> {
    let uid: u64 = match login_id.parse() {
        Ok(id) => id,
        Err(_) => return R::error(400, "无效的用户ID格式".into()),
    };

    // get_by_id(executor, id_expr) —— executor 在前，id 值在后
    match crate::models::User::get_by_id(&mut db, uid).await {
        Ok(user) => R::ok(UserInfo::from(user)),
        Err(e) => R::error(500, format!("查询用户失败: {}", e)),
    }
}

// ============ 用户 CRUD API ============

/// POST /api/v1/users — 创建用户
pub async fn create_user(
    State(mut db): State<Db>,
    ExtJson(req): ExtJson<CreateUserReq>,
) -> R<UserInfo> {
    // 检查用户名是否已存在
    let count = crate::models::User::filter_by_username(req.username.clone())
        .count()
        .exec(&mut db)
        .await
        .unwrap_or(0);
    if count > 0 {
        return R::error(400, "用户名已存在".to_string());
    }

    // 密码哈希
    let password_hash = hash(&req.password, DEFAULT_COST).unwrap_or_default();

    // 用 toasty create! 宏插入（User 已通过 use crate::models::User 引入）
    let user = toasty::create!(User {
        username: req.username,
        password_hash,
        nickname: req.nickname,
        roles: req.roles,
    })
    .exec(&mut db)
    .await;

    match user {
        Ok(u) => R::ok(UserInfo::from(u)),
        Err(e) => R::error(500, format!("创建用户失败: {}", e)),
    }
}

/// GET /api/v1/users — 用户列表
pub async fn list_users(State(mut db): State<Db>) -> R<Vec<UserInfo>> {
    match crate::models::User::all().exec(&mut db).await {
        Ok(users) => {
            let infos: Vec<UserInfo> = users.into_iter().map(UserInfo::from).collect();
            R::ok(infos)
        }
        Err(e) => R::error(500, format!("查询失败: {}", e)),
    }
}

/// GET /api/v1/users/:id — 用户详情
pub async fn get_user(
    Path(id): Path<String>,
    State(mut db): State<Db>,
) -> R<UserInfo> {
    let id_val: u64 = match id.parse() {
        Ok(v) => v,
        Err(_) => return R::error(400, "无效的用户ID".to_string()),
    };

    match crate::models::User::get_by_id(&mut db, id_val).await {
        Ok(user) => R::ok(UserInfo::from(user)),
        Err(e) => R::error(500, format!("查询失败: {}", e)),
    }
}

/// PUT /api/v1/users/:id — 更新用户
pub async fn update_user(
    Path(id): Path<String>,
    State(mut db): State<Db>,
    ExtJson(req): ExtJson<UpdateUserReq>,
) -> R<UserInfo> {
    let id_val: u64 = match id.parse() {
        Ok(v) => v,
        Err(_) => return R::error(400, "无效的用户ID".to_string()),
    };

    // 先查询用户
    let mut user = match crate::models::User::get_by_id(&mut db, id_val).await {
        Ok(u) => u,
        Err(e) => return R::error(500, format!("查询失败: {}", e)),
    };

    // 按字段更新
    if let Some(nickname) = req.nickname {
        user.nickname = nickname;
    }
    if let Some(roles) = req.roles {
        user.roles = roles;
    }
    if let Some(status) = req.status {
        user.status = status;
    }

    // 执行更新 — update() 返回 ()，需要重新查询获取更新后的记录
    match user.update().exec(&mut db).await {
        Ok(_) => {}
        Err(e) => return R::error(500, format!("更新失败: {}", e)),
    }

    // 重新查询更新后的用户
    match crate::models::User::get_by_id(&mut db, id_val).await {
        Ok(updated) => R::ok(UserInfo::from(updated)),
        Err(e) => R::error(500, format!("查询更新后数据失败: {}", e)),
    }
}

/// DELETE /api/v1/users/:id — 删除用户
pub async fn delete_user(
    Path(id): Path<String>,
    State(mut db): State<Db>,
) -> R<String> {
    let id_val: u64 = match id.parse() {
        Ok(v) => v,
        Err(_) => return R::error(400, "无效的用户ID".to_string()),
    };

    // delete_by_id(executor, id_value)
    match crate::models::User::delete_by_id(&mut db, id_val).await {
        Ok(_) => R::ok("用户已删除".to_string()),
        Err(e) => R::error(500, format!("删除失败: {}", e)),
    }
}
