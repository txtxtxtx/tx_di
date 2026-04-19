/// 组件的作用域
///
/// `Singleton` 单例作用域,默认
/// `Prototype` 原型作用域
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Singleton,
    Prototype,
}