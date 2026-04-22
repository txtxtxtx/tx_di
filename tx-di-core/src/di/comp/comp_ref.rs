use std::any::{Any, TypeId};
use std::pin::Pin;
use std::sync::Arc;
use crate::{BuildContext, Scope};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// 存储单元：
/// - `Factory(Arc<dyn Fn>)` → 存工厂闭包，prototype 每次注入时调用
/// - `Cached(Arc<dyn Any>)` → 已实例化的单例（擦除类型）
pub enum CompRef {
    /// 尚未实例化的工厂（prototype），直接存闭包
    Factory(Arc<dyn Fn(&mut BuildContext) -> Arc<dyn Any + Send + Sync> + Send + Sync>),
    /// 已实例化并缓存的单例
    Cached(Arc<dyn Any + Send + Sync>),
}

/// 组件描述符 trait，用于在编译期定义组件的元数据和构建逻辑。
///
/// 该 trait 由 `#[tx_comp]` 宏自动生成实现，是依赖注入框架的核心接口。
/// 每个被标记为组件的结构体都必须实现此 trait，以提供以下信息：
/// - 组件的依赖关系（通过 `DEP_IDS`）
/// - 组件的作用域（通过 `SCOPE`）
/// - 组件的构建方式（通过 `build` 方法）
///
/// # Trait 约束
///
/// - `Any`: 支持运行时类型识别，用于类型擦除和向下转型
/// - `Sized`: 编译期已知大小，确保可以存储在栈上
/// - `Send + Sync`: 支持跨线程安全传递和共享，因为组件会被存入 `Arc`
/// - `'static`: 生命周期为静态，确保组件可以长期存活
pub trait ComponentDescriptor: CompInit {
    /// 组件的依赖列表，存储为返回 `TypeId` 的函数指针数组。
    ///
    /// 每个元素是一个返回依赖类型 `TypeId` 的函数，用于在运行时解析依赖关系。
    /// 该数组由宏根据结构体字段自动生成，不包含带有 `#[tx_cst]` 属性的字段。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 如果 UserService 依赖 DbPool 和 AppConfig
    /// const DEP_IDS: &'static [fn() -> TypeId] = &[
    ///     std::any::TypeId::of::<DbPool>,
    ///     std::any::TypeId::of::<AppConfig>,
    /// ];
    /// ```
    const DEP_IDS: &'static [fn() -> TypeId];

    /// 组件的作用域，决定实例的生命周期和共享策略。
    ///
    /// - `Scope::Singleton`: 全局单例，首次注入时构建并缓存，后续注入返回相同的 `Arc<T>`
    /// - `Scope::Prototype`: 原型模式，每次注入都调用工厂函数创建新实例
    ///
    /// 该常量由 `#[tx_comp]` 宏根据属性参数自动生成，默认为 `Singleton`。
    const SCOPE: Scope;

    /// 构建组件实例。
    ///
    /// 该方法由框架在初始化阶段调用，用于创建组件实例。对于无字段的组件，
    /// 通常返回 `Self {}`；对于有依赖的组件，会通过 `ctx.inject::<T>()` 注入依赖。
    ///
    /// # 参数
    ///
    /// * `ctx` - 构建上下文，用于注入依赖组件和管理组件生命周期
    ///
    /// # 返回值
    ///
    /// 返回构建完成的组件实例
    ///
    /// # 注意
    ///
    /// 该方法仅在组件首次被需要时调用（对于 Singleton），或者每次注入时调用（对于 Prototype）。
    /// 不应该手动调用此方法，应该通过 `BuildContext::inject::<T>()` 来获取组件实例。
    fn build(ctx: &mut BuildContext) -> Self;
}

/// 组件初始化 trait，用于在依赖注入完成后执行自定义初始化逻辑。
///
/// 该 trait 允许组件在构建完成后执行同步或异步的初始化操作，例如：
/// - 建立数据库连接池
/// - 加载配置文件
/// - 注册事件监听器
/// - 预热缓存等
///
/// # Trait 约束
///
/// - `Any`: 支持运行时类型识别
/// - `Sized`: 编译期已知大小
/// - `Send + Sync`: 支持跨线程安全传递和共享
/// - `'static`: 生命周期为静态，确保可以长期存活
///
/// # 初始化顺序
///
/// 通过实现 `init_sort()` 方法可以控制组件的初始化顺序，
/// 返回值越小越先初始化。默认值为 10000。
pub trait CompInit :Any + Sized + Send + Sync + 'static{
    /// 组件内部初始化方法
    ///
    /// 在主键完成构建后、注入全局上下文之前执行
    #[allow(unused_variables)]
    fn inner_init(&mut self, ctx: &mut BuildContext){}
    /// 同步初始化方法
    #[allow(unused_variables)]
    fn init(ctx: &mut BuildContext) {}

    /// 异步初始化方法
    #[allow(unused_variables)]
    fn async_init(ctx: &mut BuildContext) -> BoxFuture<'static, ()> {
        Box::pin(async {

        })
    }

    /// 初始化排序方法
    fn init_sort() -> i32 {
        10000
    }
}