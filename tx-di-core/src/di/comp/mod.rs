pub mod comp_ref;
pub mod config;

use dashmap::DashMap;
use std::any::{Any, TypeId};
use tracing::debug;
use crate::{App, BoxFuture, CompRef, Scope};
use crate::RIE;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// 类型别名：简化复杂函数指针类型的定义
/// 构建组件函数类型
pub type StoreFactoryFn = fn(&DashMap<TypeId, CompRef>) -> Box<dyn Any + Send + Sync>;
/// 同步初始化函数类型
type InitFn = fn(Arc<App>, CancellationToken) -> RIE<()>;
/// 异步初始化函数类型
/// 
/// 使用 BoxFuture 实现动态分发，这是当前 Rust 中处理异构异步函数的标准方式
type AsyncInitFn = fn(Arc<App>, CancellationToken) -> BoxFuture;
/// 异步运行函数类型
type AsyncRunFn = fn(Arc<App>, CancellationToken) -> BoxFuture;

#[linkme::distributed_slice]
pub static COMPONENT_REGISTRY: [ComponentMeta] = [..];

/// 组件元数据，存储组件的运行时信息和依赖关系。
///
/// 该结构体由 `#[tx_comp]` 宏自动生成并注册到 `COMPONENT_REGISTRY` 中，
/// 用于在运行时进行依赖解析、拓扑排序和组件构建。
///
/// # 字段说明
///
/// - `type_id`: 返回组件类型 `TypeId` 的函数指针，用于唯一标识组件类型
/// - `deps`: 组件的依赖列表，每个元素是返回依赖类型 `TypeId` 的函数指针
/// - `name`: 组件的类型名称字符串，用于调试和错误提示
/// - `scope`: 组件的作用域（Singleton 或 Prototype），决定实例的生命周期
/// - `factory_fn`: 工厂函数，接收 `&DashMap<TypeId, CompRef>`，用于构建组件实例
/// - `impl_traits`: 该组件实现的 trait 名称列表，用于支持 trait object 注入
pub struct ComponentMeta {
    /// 返回组件类型 `TypeId` 的函数指针。
    ///
    /// 用于在运行时唯一标识组件类型，支持类型安全的依赖查找和向下转型。
    pub type_id: fn() -> TypeId,

    /// 组件的依赖列表，存储为返回 `TypeId` 的函数指针数组。
    ///
    /// 该数组包含所有通过 `Arc<T>` 注入的依赖项（不包括 `#[tx_cst]` 标记的字段）。
    /// 在拓扑排序阶段用于构建依赖图，检测循环依赖。
    pub deps: &'static [fn() -> TypeId],

    /// 组件的类型名称字符串。
    ///
    /// 用于调试输出、错误消息和日志记录，提高可读性。
    pub name: &'static str,

    /// 组件的作用域，决定实例的生命周期管理策略。
    ///
    /// - `Scope::Singleton`: 全局单例，首次注入时构建并缓存
    /// - `Scope::Prototype`: 原型模式，每次注入都创建新实例
    pub scope: Scope,

    /// 工厂函数：接收 `&DashMap<TypeId, CompRef>`，返回 `Box<dyn Any>`。
    ///
    /// 统一签名供 Singleton 和 Prototype 使用：
    /// - `auto_register_all` 阶段：Singleton 立即调用并缓存，Prototype 存为闭包
    /// - App 阶段：Prototype 调用闭包每次创建新实例
    pub factory_fn: Option<StoreFactoryFn>,

    /// 该组件实现的 trait 名称列表。
    ///
    /// 用于支持 trait object 注入。当用户使用 `#[tx_comp(as_trait = "TraitName")]` 时，
    /// 宏会将 trait 名称记录到此字段。注入时，框架可以通过此字段查找实现了特定 trait 的具体类型。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// #[tx_comp(as_trait = "UserRepository")]
    /// pub struct SqliteUserRepository { ... }
    ///
    /// // ComponentMeta {
    /// //     impl_traits: &["UserRepository"],
    /// //     ...
    /// // }
    /// ```
    pub impl_traits: &'static [fn() -> TypeId],

    /// 初始化顺序
    pub init_sort_fn: fn() -> i32,
    /// 内部初始化阶段的初始化方法
    pub init_fn: Option<InitFn>,
    /// 异步初始化函数，返回 BoxFuture 以支持动态分发
    pub async_init_fn: Option<AsyncInitFn>,
    /// 异步运行函数，返回 BoxFuture 以支持动态分发
    pub async_run_fn: Option<AsyncRunFn>,
}

/// 对组件元数据进行拓扑排序，确定组件的构建顺序。 `Kahn算法`
///
/// 该函数基于组件的依赖关系图执行拓扑排序，确保在构建组件时，
/// 其所有依赖项已经被构建并可用。如果检测到循环依赖，将触发 panic。
///
/// # 参数
///
/// - `metas`: 组件元数据切片引用，包含所有需要排序的组件信息。
///   每个元素是指向 `ComponentMeta` 的引用，提供类型 ID、依赖列表等关键信息。
///
/// # 返回值
///
/// 返回按拓扑顺序排列的 `TypeId` 向量。向量中的类型 ID 顺序保证了：
/// 对于任意组件，其所有依赖项都出现在该组件之前。
///
/// # Panics
///
/// 以下情况会触发 panic：
/// - 某个组件依赖的类型未在注册表中找到
/// - 检测到循环依赖（即存在无法解析的依赖环）
/// - 内部错误：TypeId 在名称映射中未找到
///
/// # 性能
///
/// 使用 Kahn 算法实现拓扑排序，时间复杂度为 O(V + E)，
/// 其中 V 是组件数量，E 是依赖关系数量。
/// 函数会记录排序结果和耗时到 debug 日志中。
pub fn topo_sort(metas: &[&ComponentMeta]) -> Vec<TypeId> {
    let start = std::time::Instant::now();

    let n = metas.len();

    let id_to_idx: HashMap<TypeId, (usize,&str)> = metas
        .iter()
        .enumerate()
        .map(|(i, m)| ((m.type_id)(), (i,m.name)))
        .collect();
    // 入度数组：记录每个组件被多少其他组件依赖
    let mut in_degree = vec![0usize; n];
    // 邻接表：adj[j] 存储所有依赖组件 j 的组件索引
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    // 遍历每个组件 i
    for (i, meta) in metas.iter().enumerate() {
        // 遍历组件 i 的所有依赖
        for dep_fn in meta.deps {
            // 获取依赖的类型ID
            let one_type_id = dep_fn();
            if let Some(&(j_idx, _j_name)) = id_to_idx.get(&one_type_id) {
                adj[j_idx].push(i);  // 建立边：j → i（j 被 i 依赖）
                in_degree[i] += 1;
            } else {
                let registered: Vec<&str> = metas.iter().map(|m| m.name).collect();
                panic!(
                    "[di] 拓扑排序失败: 组件 '{}' 依赖的 TypeId {:?} 未注册。\n\
                     请确认该依赖组件已标注 #[tx_comp] 且其 crate 已引入。\n\
                     已注册组件 ({} 个): [{}]",
                    meta.name,
                    one_type_id,
                    registered.len(),
                    registered.join(", ")
                );
            }
        }
    }
    // 将所有入度为 0 的节点加入优先队列（无依赖的组件）
    // 使用 BinaryHeap + Reverse 实现最小堆，按 init_sort_fn 值排序（值越小优先级越高）
    let mut heap: BinaryHeap<Reverse<(i32, usize)>> = (0..n)
        .filter(|&i| in_degree[i] == 0)
        .map(|i| Reverse(((metas[i].init_sort_fn)(), i)))
        .collect();
    let mut result = Vec::with_capacity(n);

    while let Some(Reverse((_sort_key, i))) = heap.pop() {
        // 将当前组件加入结果
        result.push((metas[i].type_id)());
        // 遍历所有依赖当前组件 i 的其他组件 j
        for &j in &adj[i] {
            in_degree[j] -= 1; // j 的一个依赖已满足，入度 -1
            if in_degree[j] == 0 { // 如果 j 的所有依赖都满足了
                heap.push(Reverse(((metas[j].init_sort_fn)(), j)));  // 将 j 加入优先队列等待处理
            }
        }
    }

    if result.len() != n {
        // 收集参与循环的组件及其依赖关系，帮助定位环路
        let cycle_details: Vec<String> = metas
            .iter()
            .enumerate()
            .filter(|(i, _)| in_degree[*i] > 0)
            .map(|(_, m)| {
                let dep_names: Vec<&str> = m.deps
                    .iter()
                    .filter_map(|dep_fn| {
                        let dep_id = dep_fn();
                        // 只显示未解析的依赖（即仍在循环中的）
                        if id_to_idx.get(&dep_id).map(|(idx, _)| in_degree[*idx] > 0).unwrap_or(false) {
                            id_to_idx.get(&dep_id).map(|(_, name)| *name)
                        } else {
                            id_to_idx.get(&dep_id).map(|(_, name)| *name)
                        }
                    })
                    .collect();
                format!("{} → [{}]", m.name, dep_names.join(", "))
            })
            .collect();
        panic!(
            "[di] 检测到循环依赖！以下组件形成环路:\n{}\n\
             请检查这些组件之间是否存在相互依赖，打破环中任意一条边即可。",
            cycle_details.join("\n")
        );
    }

    let sorted_names: Vec<&str> = result
        .iter()
        .map(|t| {
            id_to_idx.get(t).copied().unwrap_or_else(|| {
                panic!("[di] 拓扑排序内部错误：TypeId {:?} 未在名称映射中找到", t)
            })
        })
        .map(|(_, name)| name)
        .collect();
    debug!("[di] 拓扑排序结果：[{}]", sorted_names.join(", "));
    let elapsed = start.elapsed();
    debug!("[di] 拓扑排序耗时: {:?}", elapsed);

    result
}