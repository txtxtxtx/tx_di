use std::io::{Read, Result, Write};
use std::path::Path;

fn main() -> Result<()> {
    // 使用 vendored protoc，无需系统安装
    let protoc_path = protoc_bin_vendored::protoc_bin_path().unwrap();
    // SAFETY: build.rs 是单线程的，set_var 仅用于引导 protoc 路径
    unsafe { std::env::set_var("PROTOC", protoc_path); }

    // 确保生成目录存在（首次 clone 构建时需要）
    std::fs::create_dir_all("src/pb")?;

    let proto_dir = "protos";

    let proto_files = [
        "common",
        "auth",
        "user",
        "role",
        "menu",
        "department",
        "config",
        "dictionary",
        "log",
        "file",
        "job",
        "monitor",
        "tool",
    ];

    let proto_paths: Vec<String> = proto_files
        .iter()
        .map(|name| format!("{}/{}.proto", proto_dir, name))
        .collect();

    tonic_build::configure()
        .out_dir("src/pb")
        // 顺序很重要（serde_with 硬性要求）：
        // 1. `#[serde_with::serde_as]` 属性宏必须**先于** derive 出现，
        //    它才能把字段上的 `#[serde_as(as = "...")]` 注解改写为
        //    `#[serde(serialize_with = ..., deserialize_with = ...)]`。
        //    若放在 derive 之后，宏不会改写字段注解，i64/u64 字符串序列化静默失效。
        // 2. 之后才是 serde derive + rename_all helper attribute。
        .type_attribute(".", "#[serde_with::serde_as]")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .field_attribute("optional", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        // 生成文件描述符集合（供 gRPC 服务反射使用）
        .file_descriptor_set_path("src/pb/admin_descriptor.bin")
        .compile_protos(&proto_paths, &[proto_dir])?;

    // 后处理：为 i64/u64 字段添加 serde_as 属性
    let pb_dir = Path::new("src/pb");
    for entry in std::fs::read_dir(pb_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "rs") {
            post_process_serde_as(&path)?;
        }
    }

    Ok(())
}

/// 后处理：为 i64/u64 字段添加 serde_as 注解
/// - repeated → FlexibleVec（序列化为字符串数组，反序列化接受数字/字符串）
/// - optional → Option<FlexibleDisplayFromStr>
/// - 普通    → FlexibleDisplayFromStr
///
/// 使用 flexible_serde 模块的类型，序列化输出字符串（JS 安全），
/// 反序列化同时接受数字和字符串（前端兼容）。
fn post_process_serde_as(path: &Path) -> Result<()> {
    let mut content = String::new();
    std::fs::File::open(path)?.read_to_string(&mut content)?;

    let mut result = String::new();
    for line in content.lines() {
        result.push_str(line);
        result.push('\n');

        let is_i64 = line.contains("#[prost(int64,");
        let is_u64 = line.contains("#[prost(uint64,");

        if is_i64 || is_u64 {
            let indent = &line[..line.len() - line.trim_start().len()];
            let is_repeated = line.contains("repeated");
            let is_optional = line.contains("optional");

            let as_type = if is_repeated {
                "crate::flexible_serde::FlexibleVec"
            } else if is_optional {
                "Option<crate::flexible_serde::FlexibleDisplayFromStr>"
            } else {
                "crate::flexible_serde::FlexibleDisplayFromStr"
            };
            result.push_str(&format!(
                "{}#[serde_as(as = \"{}\")]\n",
                indent, as_type
            ));
        }
    }

    std::fs::File::create(path)?.write_all(result.as_bytes())?;
    Ok(())
}
