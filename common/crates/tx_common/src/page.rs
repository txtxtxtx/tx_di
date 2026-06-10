
#[derive(Debug, Clone)]
pub struct Page<T>{
    pub list: Vec<T>,
    pub page: i64,
    pub size: i64,
    pub total: i64,
}
impl <T> Page<T> {
    /// 构建分页对象
    pub fn new(list: Vec<T>, page: i64, size: i64, total: i64) -> Self {
        Self {
            list,
            page: page.max(1),
            size: size.clamp(1, 200),
            total,
        }
    }

    /// 构建分页对象
    pub fn req_page(page: i64, size: i64) -> Self {
        Self {
            list: Vec::with_capacity(size as usize),
            page: page.max(1),
            size: size.clamp(1, 200),
            total: 0,
        }
    }
}

impl<T> Default for Page<T> {
    /// 默认
    fn default() -> Self {
        Self {
            list: vec![],
            page: 1,
            size: 10,
            total: 0,
        }
    }
}