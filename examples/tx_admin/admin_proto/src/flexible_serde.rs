//! 灵活的 i64/u64 序列化/反序列化
//!
//! - **序列化**：输出字符串 `"123"`（避免 JavaScript 数值精度丢失）
//! - **反序列化**：同时接受数字 `123` 和字符串 `"123"`

use serde::{de, Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};
use std::fmt;
use std::marker::PhantomData;

/// 序列化为字符串，反序列化同时接受数字和字符串。
///
/// 用于替代 `serde_with::DisplayFromStr`，解决前端发送 `{ page: 1 }` 时
/// 后端无法反序列化的问题。
pub struct FlexibleDisplayFromStr;

// ── 序列化：与 DisplayFromStr 相同，输出字符串 ──────────────────────

impl<T> SerializeAs<T> for FlexibleDisplayFromStr
where
    T: fmt::Display,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&source.to_string())
    }
}

// ── 反序列化：接受数字或字符串 ─────────────────────────────────────

impl<'de, T> DeserializeAs<'de, T> for FlexibleDisplayFromStr
where
    T: std::str::FromStr,
    T::Err: fmt::Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FlexibleVisitor<T>(PhantomData<T>);

        impl<'de, T> de::Visitor<'de> for FlexibleVisitor<T>
        where
            T: std::str::FromStr,
            T::Err: fmt::Display,
        {
            type Value = T;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a number or a string representing a number")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<T, E> {
                v.to_string().parse().map_err(de::Error::custom)
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<T, E> {
                v.to_string().parse().map_err(de::Error::custom)
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<T, E> {
                v.to_string().parse().map_err(de::Error::custom)
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<T, E> {
                v.parse().map_err(de::Error::custom)
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<T, E> {
                v.parse().map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(FlexibleVisitor(PhantomData))
    }
}

/// 对 `Vec<T>` 的支持：序列化为字符串数组，反序列化接受数字/字符串混合数组。
pub struct FlexibleVec;

impl<T> SerializeAs<Vec<T>> for FlexibleVec
where
    T: fmt::Display,
{
    fn serialize_as<S>(source: &Vec<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(source.len()))?;
        for item in source {
            seq.serialize_element(&item.to_string())?;
        }
        seq.end()
    }
}

impl<'de, T> DeserializeAs<'de, Vec<T>> for FlexibleVec
where
    T: std::str::FromStr,
    T::Err: fmt::Display,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecVisitor<T>(PhantomData<T>);

        impl<'de, T> de::Visitor<'de> for VecVisitor<T>
        where
            T: std::str::FromStr,
            T::Err: fmt::Display,
        {
            type Value = Vec<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a sequence of numbers or strings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Vec<T>, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element::<serde_json::Value>()? {
                    let parsed = match elem {
                        serde_json::Value::Number(n) => {
                            n.to_string().parse().map_err(de::Error::custom)?
                        }
                        serde_json::Value::String(s) => {
                            s.parse().map_err(de::Error::custom)?
                        }
                        other => {
                            return Err(de::Error::custom(format!(
                                "expected number or string, got {}",
                                other
                            )))
                        }
                    };
                    vec.push(parsed);
                }
                Ok(vec)
            }
        }

        deserializer.deserialize_seq(VecVisitor(PhantomData))
    }
}
