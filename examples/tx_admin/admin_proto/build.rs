use std::io::Result;

fn main() -> Result<()> {
    // 使用 vendored protoc，无需系统安装
    let protoc_path = protoc_bin_vendored::protoc_bin_path().unwrap();
    // SAFETY: build.rs 是单线程的，set_var 仅用于引导 protoc 路径
    unsafe { std::env::set_var("PROTOC", protoc_path); }

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
    ];

    let proto_paths: Vec<String> = proto_files
        .iter()
        .map(|name| format!("{}/{}.proto", proto_dir, name))
        .collect();

    tonic_build::configure()
        // 为所有 message 类型添加 Serialize/Deserialize，使 HTTP JSON 可用
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        // 为所有 message 添加 serde rename_all = "camelCase"
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        // prost 的 optional 字段 -> Option<T>，跳过 None 值
        // 注意：不能全局 field_attribute(".")，因为 Vec<T> 不支持 Option::is_none
        .field_attribute("optional", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        // uint64 在 JSON 中应序列化为字符串（JS 精度问题）
        .field_attribute("uint64", "#[serde(with = \"crate::serde_u64\")]")
        // proto 文件所在目录，用于 import 解析
        .compile_protos(&proto_paths, &[proto_dir])?;

    Ok(())
}
