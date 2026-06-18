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
        "permission",
        "config",
        "dictionary",
        "log",
        "file",
        "monitor",
        "tool",
    ];

    let proto_paths: Vec<String> = proto_files
        .iter()
        .map(|name| format!("{}/{}.proto", proto_dir, name))
        .collect();

    tonic_build::configure()
        .out_dir("src/pb")
        // 顺序很重要：
        // 1. schemars 先展开，看到原始 struct（无 phantom 字段）
        // 2. serde(rename_all) 同时被 schemars 和 serde 读取
        // 3. serde_as 展开，注入 phantom 字段和 serde(with) 属性
        // 4. Serialize/Deserialize derive 展开，看到 DisplayFromStr 生效
        .type_attribute(".", "#[derive(schemars::JsonSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .type_attribute(".", "#[serde_with::serde_as]")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute("optional", "#[serde(skip_serializing_if = \"Option::is_none\")]")
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
/// - repeated i64/u64 字段 → #[serde_as(as = "Vec<serde_with::DisplayFromStr>")]
/// - optional i64/u64 字段 → #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
/// - 普通 i64/u64 字段 → #[serde_as(as = "serde_with::DisplayFromStr")]
fn post_process_serde_as(path: &Path) -> Result<()> {
    let mut content = String::new();
    std::fs::File::open(path)?.read_to_string(&mut content)?;

    let mut result = String::new();
    for line in content.lines() {
        result.push_str(line);
        result.push('\n');

        let is_i64 = line.contains("#[prost(int64,");
        let is_u64 = line.contains("#[prost(uint64,") || line.contains("#[prost(uint64)]");

        if is_i64 || is_u64 {
            let indent = &line[..line.len() - line.trim_start().len()];
            let is_repeated = line.contains("repeated");
            let is_optional = line.contains("optional");

            let as_type = if is_repeated {
                "Vec<serde_with::DisplayFromStr>"
            } else if is_optional {
                "Option<serde_with::DisplayFromStr>"
            } else {
                "serde_with::DisplayFromStr"
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
