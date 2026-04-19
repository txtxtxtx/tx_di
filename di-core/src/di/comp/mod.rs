pub mod comp_ref;

use std::any::{Any, TypeId};
use log::debug;
use crate::{BuildContext, Scope};

#[linkme::distributed_slice]
pub static COMPONENT_REGISTRY: [ComponentMeta] = [..];

pub struct ComponentMeta {
    pub type_id: fn() -> TypeId,
    pub deps: &'static [fn() -> TypeId],
    pub name: &'static str,
    pub scope: Scope,
    /// 原始工厂函数（用于 `debug_registry` 诊断；运行时不使用）
    pub factory_fn: Option<fn(&mut BuildContext) -> Box<dyn Any + Send + Sync>>,
}

/// 拓扑排序
pub fn topo_sort(metas: &[&ComponentMeta]) -> Vec<TypeId> {
    use std::collections::{HashMap, VecDeque};

    let n = metas.len();

    let id_to_name: HashMap<TypeId, &str> = metas
        .iter()
        .map(|m| ((m.type_id)(), m.name))
        .collect();

    let id_to_idx: HashMap<TypeId, usize> = metas
        .iter()
        .enumerate()
        .map(|(i, m)| ((m.type_id)(), i))
        .collect();

    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];

    for (i, meta) in metas.iter().enumerate() {
        for dep_fn in meta.deps {
            let one_type_id = dep_fn();
            if let Some(&j) = id_to_idx.get(&one_type_id) {
                adj[j].push(i);
                in_degree[i] += 1;
            } else {
                panic!(
                    "[di] 组件 '{}' 依赖的类型 {:?} {:?} 未在注册表中找到",
                    meta.name,
                    id_to_name.get(&one_type_id),
                    &one_type_id
                );
            }
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut result = Vec::with_capacity(n);

    while let Some(i) = queue.pop_front() {
        result.push((metas[i].type_id)());
        for &j in &adj[i] {
            in_degree[j] -= 1;
            if in_degree[j] == 0 {
                queue.push_back(j);
            }
        }
    }

    if result.len() != n {
        let cycles: Vec<&str> = metas
            .iter()
            .enumerate()
            .filter(|(i, _)| in_degree[*i] > 0)
            .map(|(_, m)| m.name)
            .collect();
        panic!("[di] 循环依赖：{:?}", cycles);
    }

    let sorted_names: Vec<&str> = result
        .iter()
        .map(|t| {
            id_to_name.get(t).copied().unwrap_or_else(|| {
                panic!("[di] 拓扑排序内部错误：TypeId {:?} 未在名称映射中找到", t)
            })
        })
        .collect();
    debug!("[di] 拓扑排序结果：\n[\n{}\n]", sorted_names.join(",\n"));

    result
}