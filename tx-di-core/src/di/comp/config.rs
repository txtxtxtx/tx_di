use std::any::TypeId;
use std::fs;
use std::path::{Path, PathBuf};
use serde::de::DeserializeOwned;
use toml::Value::Table;
use crate::{BuildContext, CompInit, ComponentDescriptor, Scope};

/// 全局配置文件
pub struct AppAllConfig{
    pub toml_value: toml::Value,
}
impl AppAllConfig {
    pub fn new<P: Into<PathBuf>>(config_path: Option<P>) -> Self {
        // 如果提供了配置文件路径，从配置文件加载组件
        // 确定配置文件路径
        let final_config_path = if let Some(path) = config_path {
            path.into()
        } else {
            // 默认使用可执行文件所在目录的 config/config.toml
            let exe_path = std::env::current_exe().unwrap_or_else(|e| {
                eprintln!("[di] 警告：无法获取可执行文件路径: {}", e);
                PathBuf::from(".")
            });

            let config_dir = exe_path.parent().unwrap_or_else(|| {
                eprintln!("[di] 警告：无法获取可执行文件父目录");
                Path::new(".")
            }).join("config");

            config_dir.join("config.toml")
        };
        let toml_value = Self::load_config(final_config_path.as_path());
        AppAllConfig {
            toml_value,
        }
    }
    /// 加载配置文件（如果存在）
    fn load_config(path: &Path) -> toml::Value {
        let config = Table(toml::map::Map::new());
        if !path.exists() {
            eprintln!("[di] 配置文件不存在: {:?}，将使用默认配置", path);
            return config;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[di] 警告：无法读取配置文件 '{:?}': {}", path, e);
                return config;
            }
        };

        // 解析 TOML
        let config: toml::Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[di] 警告：配置文件 '{:?}' 解析失败: {}", path, e);
                return config;
            }
        };
        // 配置文件已加载，可以在这里将配置存储到上下文中
        eprintln!("[di] 配置文件加载成功: {:?}", path);
        config
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let value = self.get_value(key)?;
        T::deserialize(value.clone()).ok()
    }
    pub fn get_or_default<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    pub fn get_value(&self, key: &str) -> Option<&toml::Value> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = &self.toml_value;
        for k in keys {
            current = current.get(k)?;
        }
        Some(current)
    }
}

impl CompInit for AppAllConfig {}

impl ComponentDescriptor for AppAllConfig {
    const DEP_IDS: &'static [fn() -> TypeId] = &[];
    const SCOPE: Scope = Scope::Singleton;

    fn build(ctx: &mut BuildContext) -> Self {
        panic!("AppAllConfig should not be built via ComponentDescriptor::build. It is manually created in BuildContext::new().")
    }
}