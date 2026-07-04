//! 全局配置管理
//!
//! 从 TOML 配置文件加载配置，支持点分路径访问。
//! 配置组件通过 `#[component(conf = "key")]` 自动反序列化。

use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use toml::Value::Table;

use crate::component::Component;
use crate::scope::Scope;

/// 全局配置文件
pub struct AppAllConfig {
    pub toml_value: toml::Value,
}

impl AppAllConfig {
    /// 从指定路径或默认路径加载配置
    pub fn new<P: Into<PathBuf>>(config_path: Option<P>) -> Self {
        let final_config_path = if let Some(path) = config_path {
            path.into()
        } else {
            // 默认使用可执行文件所在目录的 config/config.toml
            let exe_path = std::env::current_exe().unwrap_or_else(|e| {
                panic!(
                    "[di] 无法获取可执行文件路径: {}。\n\
                     请检查程序运行环境，或手动传入配置路径。",
                    e
                )
            });

            let config_dir = exe_path
                .parent()
                .unwrap_or_else(|| {
                    panic!(
                        "[di] 无法获取可执行文件父目录: {:?}。\n\
                         请手动传入配置路径。",
                        exe_path
                    )
                })
                .join("config");

            config_dir.join("config.toml")
        };

        crate::lifecycle::set_sys_config(
            crate::lifecycle::CONFIG_PATH,
            final_config_path.to_str().unwrap().to_string(),
        );
        let toml_value = Self::load_config(final_config_path.as_path());
        AppAllConfig { toml_value }
    }

    /// 加载配置文件（如果存在）
    ///
    /// 配置文件不存在时返回空 Table（允许无配置运行）。
    /// 配置文件存在但读取/解析失败时 panic，避免使用错误的默认值。
    fn load_config(path: &Path) -> toml::Value {
        if !path.exists() {
            eprintln!("[di] 配置文件不存在: {:?}，将使用默认配置", path);
            return Table(toml::map::Map::new());
        }

        let content = fs::read_to_string(path).unwrap_or_else(|e| {
            panic!(
                "[di] 配置文件读取失败: {:?}\n\
                 错误: {}\n\
                 请检查文件权限和路径是否正确。",
                path, e
            )
        });

        let config: toml::Value = toml::from_str(&content).unwrap_or_else(|e| {
            panic!(
                "[di] 配置文件解析失败: {:?}\n\
                 错误: {}\n\
                 请检查 TOML 语法是否正确。",
                path, e
            )
        });
        eprintln!("[di] 配置文件加载成功: {:?}", path);
        config
    }

    /// 获取配置值并反序列化
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let value = self.get_value(key)?;
        T::deserialize(value.clone()).ok()
    }

    /// 获取配置值或默认值
    pub fn get_or_default<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// 获取原始 TOML 值
    pub fn get_value(&self, key: &str) -> Option<&toml::Value> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = &self.toml_value;
        for k in keys {
            current = current.get(k)?;
        }
        Some(current)
    }
}

// AppAllConfig 特殊处理：不走标准 Component 流程
// 它在 BuildContext::new() 阶段直接构造并放入 Store
impl Component for AppAllConfig {
    type Deps = ();

    fn build(_deps: Self::Deps) -> Self {
        panic!("[di] AppAllConfig 只在 BuildContext::new() 内部构建，不应通过 Component::build() 调用")
    }

    const SCOPE: Scope = Scope::Singleton;
}
