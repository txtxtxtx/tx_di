//! 实体基类 trait

/// 所有实体的公共基类
///
/// 每个实体拥有一个唯一标识 `Id`，由具体聚合根实现。
pub trait Entity {
    type Id: Copy + Eq + std::hash::Hash;
    fn id(&self) -> Self::Id;
}
