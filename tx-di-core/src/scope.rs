//! 组件作用域
//!
//! - `Singleton`：全局单例，工厂调用一次，缓存 `Arc<T>`
//! - `Prototype`：每次注入调用工厂，构造新实例

/// 组件的作用域
///
/// `Singleton` 单例作用域，默认
/// `Prototype` 原型作用域
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// 全局单例：首次注入时构建并缓存，后续返回同一个 `Arc<T>`
    Singleton,
    /// 原型：每次注入都调用工厂创建新实例
    Prototype,
}

impl Scope {
    /// 是否为单例
    pub fn is_singleton(&self) -> bool {
        matches!(self, Scope::Singleton)
    }

    /// 是否为原型
    pub fn is_prototype(&self) -> bool {
        matches!(self, Scope::Prototype)
    }
}

impl Default for Scope {
    fn default() -> Self {
        Scope::Singleton
    }
}
