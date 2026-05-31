use std::fmt;
use serde::{Serialize, Serializer, Deserialize, Deserializer, de};
use chrono::{Local, Utc};
use serde::de::Visitor;

/// 格式化日期时间包装器
///
/// 在序列化时自动将 DateTime 格式化为指定格式的字符串
/// `as_timestamp` 序列化为时间戳（秒）
/// # **`as_timestamp_millis`** 序列化为时间戳（毫秒）数据库存储毫秒，反序列化也是毫秒
/// 使用
///
/// ```rust,ignore
/// #[serde(with = "as_timestamp_millis")] // 序列化为时间戳毫秒 
/// pub aaa: FormattedDateTime
/// #[serde(with = "as_timestamp")] // 序列化为时间戳秒
/// pub aaa: FormattedDateTime
///

#[derive(Debug, Clone)]
pub struct FormattedDateTime(pub chrono::DateTime<Local>);

impl FormattedDateTime {
    /// 创建新的格式化日期时间
    pub fn new(dt: chrono::DateTime<Local>) -> Self {
        Self(dt)
    }

    /// 获取当前时间的格式化包装
    pub fn now() -> Self {
        Self(Local::now())
    }

    /// 获取内部存储的 UTC 时间
    pub fn to_utc(&self) -> chrono::DateTime<Utc> {
        self.0.with_timezone(&Utc)
    }

    /// 转换为 Unix 时间戳（秒），用于数据库存储
    pub fn as_timestamp(&self) -> i64 {
        self.0.timestamp()
    }

    /// 转换为 Unix 时间戳（毫秒），用于高精度数据库存储
    pub fn as_timestamp_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }

    /// 从 Unix 时间戳（秒）创建
    pub fn from_timestamp(secs: i64) -> Option<Self> {
        chrono::DateTime::from_timestamp(secs, 0)
            .map(|dt| Self(dt.with_timezone(&Local)))
    }

    /// 从 Unix 时间戳（毫秒）创建
    pub fn from_timestamp_millis(millis: i64) -> Option<Self> {
        let secs = millis / 1000;
        let nsecs = ((millis % 1000) * 1_000_000) as u32;
        chrono::DateTime::from_timestamp(secs, nsecs)
            .map(|dt| Self(dt.with_timezone(&Local)))
    }
}




impl Serialize for FormattedDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = self.0.format("%Y-%m-%d %H:%M:%S").to_string();
        serializer.serialize_str(&formatted)
    }
}

impl<'de> Deserialize<'de> for FormattedDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FormattedDateTimeVisitor;

        impl<'de> Visitor<'de> for FormattedDateTimeVisitor {
            type Value = FormattedDateTime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("一个日期时间字符串（格式：YYYY-MM-DD HH:MM:SS，本地时间）或时间戳")
            }

            // 支持从 i64 时间戳反序列化（某些数据库场景）
            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let secs = value / 1000;
                let nsecs = ((value % 1000) * 1_000_000) as u32;
                let dt = chrono::DateTime::from_timestamp(secs, nsecs)
                    .ok_or_else(|| de::Error::custom("无效的毫秒时间戳"))?
                    .with_timezone(&Local);
                Ok(FormattedDateTime(dt))
            }

            // 支持从 u64 时间戳反序列化
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let secs = (value / 1000) as i64;
                let nsecs = ((value % 1000) * 1_000_000) as u32;
                let dt = chrono::DateTime::from_timestamp(secs, nsecs)
                    .ok_or_else(|| de::Error::custom("无效的时间戳"))?
                    .with_timezone(&Local);
                Ok(FormattedDateTime(dt))
            }

            // 支持从字符串反序列化（JSON 场景）
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // 将输入的本地时间字符串解析为 NaiveDateTime
                let naive = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| de::Error::custom(format!("日期格式错误: {}", e)))?;

                // 附加本地时区，处理夏令时边界情况
                let dt = match naive.and_local_timezone(Local) {
                    chrono::LocalResult::Single(dt) => dt,
                    chrono::LocalResult::Ambiguous(earliest, _latest) => {
                        // 夏令时回拨时的歧义时间，选择较早的那个
                        earliest
                    },
                    chrono::LocalResult::None => {
                        return Err(de::Error::custom(
                            format!("时间 '{}' 在本地时区不存在（可能是夏令时跳过的时间）", value)
                        ));
                    }
                };
                Ok(FormattedDateTime(dt))
            }
        }

        deserializer.deserialize_any(FormattedDateTimeVisitor)
    }
}

impl From<chrono::DateTime<Local>> for FormattedDateTime {
    fn from(dt: chrono::DateTime<Local>) -> Self {
        Self(dt)
    }
}

impl From<chrono::DateTime<Utc>> for FormattedDateTime {
    fn from(dt: chrono::DateTime<Utc>) -> Self {
        Self(dt.with_timezone(&Local))
    }
}

impl std::ops::Deref for FormattedDateTime {
    type Target = chrono::DateTime<Local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


pub mod as_timestamp {
    use super::*;
    use serde::{Serializer, Deserializer};

    #[allow(dead_code)]
    pub fn serialize<S>(dt: &FormattedDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(dt.as_timestamp())
    }
    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<FormattedDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = i64::deserialize(deserializer)?;
        FormattedDateTime::from_timestamp(timestamp)
            .ok_or_else(|| de::Error::custom("无效的时间戳"))
    }
}

/// 用于数据库存储的时间戳（毫秒）序列化模块
pub mod as_timestamp_millis {
    use super::*;
    use serde::{Serializer, Deserializer};
    #[allow(dead_code)]
    pub fn serialize<S>(dt: &FormattedDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(dt.as_timestamp_millis())
    }
    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<FormattedDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = i64::deserialize(deserializer)?;
        FormattedDateTime::from_timestamp_millis(timestamp)
            .ok_or_else(|| de::Error::custom("无效的时间戳（毫秒）"))
    }
}