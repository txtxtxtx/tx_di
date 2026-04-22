我来详细解析这个拓扑排序算法。它是一个**带优先级的 Kahn 算法**，用于解决依赖注入框架中组件的初始化顺序问题。

## 核心问题

在 DI 框架中，组件之间存在依赖关系：
- `ServiceA` 依赖 `DatabasePool`
- `ServiceB` 依赖 `ServiceA` 和 `Cache`

必须按正确顺序初始化——**被依赖的组件先初始化**。

---

## 数据结构解析

```
ComponentMeta {
    type_id: fn() -> TypeId,   // 组件唯一标识
    deps: &[fn() -> TypeId],   // 该组件依赖的其他组件列表
    init_sort_fn: fn() -> i32, // 优先级权重（越小越先初始化）
    ...
}
```

---

## 算法分五个阶段

### 阶段一：建立 TypeId → 索引 的映射

```rust
let id_to_idx: HashMap<TypeId, (usize, &str)> = metas
    .iter()
    .enumerate()
    .map(|(i, m)| ((m.type_id)(), (i, m.name)))
    .collect();
```

把每个组件的 `TypeId` 映射到它在 `metas` 数组中的下标，方便后续 O(1) 查找。

```
TypeId(DatabasePool) → (0, "DatabasePool")
TypeId(Cache)        → (1, "Cache")
TypeId(ServiceA)     → (2, "ServiceA")
TypeId(ServiceB)     → (3, "ServiceB")
```

---

### 阶段二：构建有向图（邻接表 + 入度表）

```rust
for (i, meta) in metas.iter().enumerate() {
    for dep_fn in meta.deps {
        let one_type_id = dep_fn();
        if let Some(&j) = id_to_idx.get(&one_type_id) {
            adj[j.0].push(i);   // 边：j → i
            in_degree[i] += 1;
        }
    }
}
```

**关键理解：边的方向**

> "A 依赖 B" → 建边 **B → A**（B 指向 A）

这个方向意味着：**当 B 处理完毕后，才能触发 A 进入可处理状态**。

以具体例子说明：

```
ServiceA.deps = [DatabasePool]
ServiceB.deps = [ServiceA, Cache]
```

| 遍历 | 含义 | 建边 | 入度变化 |
|------|------|------|----------|
| ServiceA 依赖 DatabasePool | DatabasePool 要先于 ServiceA | DatabasePool→ServiceA | in_degree[ServiceA]++ |
| ServiceB 依赖 ServiceA | ServiceA 要先于 ServiceB | ServiceA→ServiceB | in_degree[ServiceB]++ |
| ServiceB 依赖 Cache | Cache 要先于 ServiceB | Cache→ServiceB | in_degree[ServiceB]++ |

构建结果：

```
adj[0] = [2]        // DatabasePool → ServiceA
adj[1] = [3]        // Cache → ServiceB
adj[2] = [3]        // ServiceA → ServiceB
adj[3] = []         

in_degree = [0, 0, 1, 2]
             DB  Ca  SA  SB
```

---

### 阶段三：初始化优先级最小堆

```rust
let mut heap: BinaryHeap<Reverse<(i32, usize)>> = (0..n)
    .filter(|&i| in_degree[i] == 0)      // 只取无依赖的节点
    .map(|i| Reverse(((metas[i].init_sort_fn)(), i)))  // 附带排序权重
    .collect();
```

- `BinaryHeap` 默认是最大堆，套上 `Reverse` 变成**最小堆**
- 堆元素是 `(sort_key, index)`，先按 `sort_key` 排序，相同时按 `index`
- 初始只有入度为 0 的节点进堆（`DatabasePool`、`Cache`）

```
heap: [(sort_key_DB, 0), (sort_key_Cache, 1)]
```

---

### 阶段四：Kahn 算法主循环

```rust
while let Some(Reverse((_sort_key, i))) = heap.pop() {
    result.push((metas[i].type_id)());   // ① 当前节点加入结果
    for &j in &adj[i] {
        in_degree[j] -= 1;               // ② 后继节点入度 -1
        if in_degree[j] == 0 {
            heap.push(Reverse(((metas[j].init_sort_fn)(), j)));  // ③ 新的零入度节点入堆
        }
    }
}
```

完整执行过程：

```
初始堆: [(w_DB, 0:DB), (w_Ca, 1:Cache)]  假设 w_DB < w_Ca

Step 1: pop DB(0)
  result = [TypeId(DB)]
  adj[0] = [2:ServiceA] → in_degree[2] = 1-1 = 0 → 入堆
  堆: [(w_Ca, 1), (w_SA, 2)]

Step 2: pop 较小权重者，假设 Cache(1)
  result = [TypeId(DB), TypeId(Cache)]
  adj[1] = [3:ServiceB] → in_degree[3] = 2-1 = 1 → 不入堆
  堆: [(w_SA, 2)]

Step 3: pop ServiceA(2)
  result = [TypeId(DB), TypeId(Cache), TypeId(SA)]
  adj[2] = [3:ServiceB] → in_degree[3] = 1-1 = 0 → 入堆
  堆: [(w_SB, 3)]

Step 4: pop ServiceB(3)
  result = [TypeId(DB), TypeId(Cache), TypeId(SA), TypeId(SB)]
  堆空，循环结束
```

---

### 阶段五：循环依赖检测

```rust
if result.len() != n {
    // 有节点没被处理 = 存在环
    let cycles = metas.iter()
        .filter(|(i, _)| in_degree[*i] > 0)  // 入度仍 > 0 的就是环中节点
        .map(|(_, m)| m.name)
        .collect();
    panic!("[di] 循环依赖：{:?}", cycles);
}
```

如果存在循环依赖（A→B→A），这些节点的入度永远不会降为 0，永远不会进堆，最终 `result.len() < n`，触发 panic。

---

## 整体流程图

```
metas[] 输入
    │
    ▼
建立 TypeId→idx 映射
    │
    ▼
构建有向图
  in_degree[]  ←── 记录每个组件"被几个组件依赖"
  adj[]        ←── 邻接表（被依赖者 → 依赖者）
    │
    ▼
零入度节点 → 最小堆（按 init_sort_fn 权重）
    │
    ▼
Kahn 主循环
  pop 堆顶 → 加入 result
  → 后继节点入度-1
  → 新零入度节点入堆
    │
    ▼
result.len == n?
  No  → panic 循环依赖
  Yes → 返回有序 TypeId 列表
```

---

## 算法特性总结

| 特性 | 说明 |
|------|------|
| **算法基础** | Kahn's Algorithm（BFS 拓扑排序） |
| **时间复杂度** | O((V+E) log V)，V=组件数，E=依赖边数 |
| **优先级控制** | `init_sort_fn` 返回 i32，相同层级内按权重决定初始化顺序 |
| **循环依赖检测** | 自动检测，`result.len() != n` 时 panic 并报出环中节点 |
| **确定性** | 相同输入 + 相同权重 → 相同输出顺序（堆保证） |
| **应用场景** | DI 容器组件初始化、模块加载、任务调度 |

`init_sort_fn` 是这个实现相对于标准 Kahn 算法的最大亮点——它允许在拓扑约束满足的前提下，进一步细化同层级组件的初始化顺序，比如让基础设施组件（DB、Redis）总是优先于业务组件初始化。