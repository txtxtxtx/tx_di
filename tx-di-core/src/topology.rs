//! 拓扑排序 — Kahn 算法
//!
//! 对 `COMPONENT_REGISTRY` 中的组件按依赖关系排序，
//! 确保构建时依赖项已就绪。检测到循环依赖时 panic。

use std::any::TypeId;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

use tracing::debug;

use crate::registry::{ComponentMeta, COMPONENT_REGISTRY};
use crate::store::TRAIT_IMPL_MAP;
use crate::error::{AppError, DiErr};

/// 对组件元数据进行拓扑排序，返回排序后的 TypeId 列表
///
/// # Panics
///
/// - 某个组件依赖的类型未在注册表中找到
/// - 检测到循环依赖
pub fn topo_sort(metas: &[&ComponentMeta]) -> Result<Vec<TypeId>, AppError> {
    let start = std::time::Instant::now();

    let n = metas.len();

    let id_to_idx: HashMap<TypeId, (usize, &str)> = metas
        .iter()
        .enumerate()
        .map(|(i, m)| ((m.type_id)(), (i, m.name)))
        .collect();

    // 入度数组：记录每个组件被多少其他组件依赖
    let mut in_degree = vec![0usize; n];
    // 邻接表：adj[j] 存储所有依赖组件 j 的组件索引
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];

    // 遍历每个组件 i，解析其依赖
    for (i, meta) in metas.iter().enumerate() {
        for dep_fn in meta.dep_type_ids {
            let one_type_id = dep_fn();
            if let Some(&(j_idx, _j_name)) = id_to_idx.get(&one_type_id) {
                // 直接匹配具体类型
                adj[j_idx].push(i);
                in_degree[i] += 1;
            } else if let Some(entries) = TRAIT_IMPL_MAP.get(&one_type_id) {
                // 依赖是 trait：通过 TRAIT_IMPL_MAP 解析出具体实现
                for entry in entries.iter() {
                    let concrete_id = (entry.concrete_tid)();
                    if let Some(&(j_idx, _j_name)) = id_to_idx.get(&concrete_id) {
                        adj[j_idx].push(i);
                        in_degree[i] += 1;
                    }
                }
            } else {
                let registered: Vec<&str> = metas.iter().map(|m| m.name).collect();
                return Err(AppError::with_context(
                    DiErr::RegistryError,
                    format!(
                        "组件 '{}' 依赖的 TypeId {:?} 未注册。\n\
                         请确认该依赖组件已标注 #[derive(Component)] 且其 crate 已引入。\n\
                         已注册组件 ({} 个): [{}]",
                        meta.name,
                        one_type_id,
                        registered.len(),
                        registered.join(", ")
                    ),
                ));
            }
        }
    }

    // 将所有入度为 0 的节点加入优先队列（无依赖的组件）
    // 使用 BinaryHeap + Reverse 实现最小堆，按 init_sort 值排序（值越小优先级越高）
    let mut heap: BinaryHeap<Reverse<(i32, usize)>> = (0..n)
        .filter(|&i| in_degree[i] == 0)
        .map(|i| Reverse((0_i32, i))) // init_sort 移到 Component trait，这里用 0
        .collect();

    let mut result = Vec::with_capacity(n);

    while let Some(Reverse((_sort_key, i))) = heap.pop() {
        result.push((metas[i].type_id)());
        for &j in &adj[i] {
            in_degree[j] -= 1;
            if in_degree[j] == 0 {
                heap.push(Reverse((0_i32, j)));
            }
        }
    }

    if result.len() != n {
        // 收集参与循环的组件及其依赖关系
        let cycle_details: Vec<String> = metas
            .iter()
            .enumerate()
            .filter(|(i, _)| in_degree[*i] > 0)
            .map(|(_, m)| {
                let dep_names: Vec<&str> = m
                    .dep_type_ids
                    .iter()
                    .filter_map(|dep_fn| {
                        let dep_id = dep_fn();
                        id_to_idx.get(&dep_id).map(|(_, name)| *name)
                    })
                    .collect();
                format!("{} → [{}]", m.name, dep_names.join(", "))
            })
            .collect();

        return Err(AppError::with_context(
            DiErr::RegistryError,
            format!(
                "检测到循环依赖！以下组件形成环路:\n{}\n\
                 请检查这些组件之间是否存在相互依赖，打破环中任意一条边即可。",
                cycle_details.join("\n")
            ),
        ));
    }

    let sorted_names: Vec<&str> = result
        .iter()
        .map(|t| {
            id_to_idx
                .get(t)
                .copied()
                .map(|(_, name)| name)
                .unwrap_or("?")
        })
        .collect();
    debug!("[di] 拓扑排序结果：[{}]", sorted_names.join(", "));
    debug!("[di] 拓扑排序耗时: {:?}", start.elapsed());

    Ok(result)
}

/// 获取所有已注册组件的元数据引用
pub fn all_metas() -> Vec<&'static ComponentMeta> {
    COMPONENT_REGISTRY.iter().collect()
}
