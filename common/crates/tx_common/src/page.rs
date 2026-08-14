use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    #[serde(default)]
    pub list: Vec<T>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_size")]
    pub size: i64,
    #[serde(default)]
    pub total: i64,
}

fn default_page() -> i64 {
    1
}
fn default_size() -> i64 {
    10
}
impl<T> Page<T> {
    /// 构造完整的分页结果
    pub fn new(list: Vec<T>, page: i64, size: i64, total: i64) -> Self {
        Self {
            list,
            page: page.max(1),
            size: size.clamp(1, 200),
            total,
        }
    }

    /// 构造仅含分页参数的空结果（用于发起查询请求）
    pub fn request(page: i64, size: i64) -> Self {
        Self {
            list: Vec::with_capacity(size as usize),
            page: page.max(1),
            size: size.clamp(1, 200),
            total: 0,
        }
    }

    /// 将 `(列表, 总数)` 填入当前分页请求对象，生成分页响应。
    ///
    /// 配合 `tx_di_toasty::toasty_page!` 宏使用：
    /// ```rust,ignore
    /// let (rows, total) = tx_di_toasty::toasty_page!(db, page, { ... }, |e| ...?);
    /// // 行 → 领域对象转换（同步或异步）后：
    /// Ok(page.fill(list, total))
    /// ```
    ///
    /// 等价于 `Page::new(list, page, size, total)`，但复用已有 `page`/`size`，
    /// 免去重复书写分页参数。
    pub fn fill(self, list: Vec<T>, total: i64) -> Self {
        Self::new(list, self.page, self.size, total)
    }

    /// 计算 SQL OFFSET 值
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.size
    }

    /// 总页数
    pub fn total_pages(&self) -> i64 {
        if self.total == 0 {
            return 0;
        }
        (self.total + self.size - 1) / self.size
    }

    /// 是否有下一页
    pub fn has_next(&self) -> bool {
        self.page * self.size < self.total
    }

    /// 是否有上一页
    pub fn has_previous(&self) -> bool {
        self.page > 1
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self::request(1, 10)
    }
}
