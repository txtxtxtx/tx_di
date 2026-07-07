//! 审计报表导出（HTML 自包含页面 + 极简手写 PDF）
//!
//! - `gen_html`：生成可直接在浏览器打开/打印为 PDF 的 HTML 报告。
//! - `gen_pdf`：不依赖第三方库，手写一个符合规范的单/多页 PDF（仅 ASCII，非 ASCII 转 `?`）。

use crate::audit::AuditEntry;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// 生成 HTML 报告内容
pub fn gen_html(title: &str, entries: &[AuditEntry]) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut rows = String::new();
    for e in entries {
        let time = format_time(e.ts_ms);
        let detail = escape_html(&e.detail);
        let result = if e.result == "ok" {
            "<span class='ok'>OK</span>".to_string()
        } else {
            format!("<span class='fail'>{}</span>", escape_html(&e.result))
        };
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            time, e.kind, detail, result
        ));
    }
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="utf-8">
<title>{title}</title>
<style>
 body{{font-family:-apple-system,Segoe UI,Roboto,'Microsoft YaHei',sans-serif;margin:32px;color:#222}}
 h1{{font-size:20px}} .meta{{color:#666;font-size:12px;margin-bottom:12px}}
 table{{border-collapse:collapse;width:100%;font-size:13px}}
 th,td{{border:1px solid #ddd;padding:6px 8px;text-align:left}}
 th{{background:#f5f5f5}} .ok{{color:#0a0}} .fail{{color:#c00}}
</style></head>
<body>
<h1>{title}</h1>
<div class="meta">生成时间：{gen}　共 {count} 条操作记录</div>
<table><thead><tr><th>时间</th><th>类型</th><th>详情</th><th>结果</th></tr></thead>
<tbody>
{rows}</tbody></table>
</body></html>"#,
        title = escape_html(title),
        gen = format_time(ts),
        count = entries.len(),
        rows = rows,
    )
}

/// 导出 HTML 报告到文件
pub fn export_html(path: &str, title: &str, entries: &[AuditEntry]) -> std::io::Result<()> {
    fs::write(path, gen_html(title, entries))
}

/// 导出 PDF 报告到文件（极简手写 PDF，按行分页）
pub fn export_pdf(path: &str, title: &str, entries: &[AuditEntry]) -> std::io::Result<()> {
    let lines: Vec<String> = {
        let mut v = vec![format!("== {} ==", ascii_only(title)), format!("Total: {}", entries.len())];
        for e in entries {
            v.push(format!(
                "[{}] {} | {} | {}",
                format_time(e.ts_ms),
                e.kind,
                ascii_only(&e.detail),
                ascii_only(&e.result)
            ));
        }
        v
    };
    let pdf = build_pdf(&lines);
    fs::write(path, pdf)
}

/// 构造一个分页 PDF（每页 ~50 行）
fn build_pdf(lines: &[String]) -> Vec<u8> {
    let per_page = 50usize;
    let pages: Vec<&[String]> = lines.chunks(per_page).collect();
    let n_pages = pages.len().max(1);

    // 对象编号：1=catalog, 2=pages, 3..=3+n-1 = page, 然后 font, 然后 contents
    let font_obj = 3 + n_pages;
    let first_content = font_obj + 1;

    let mut objects: Vec<String> = Vec::new();
    // 1 catalog
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());
    // 2 pages
    let kids: Vec<String> = (0..n_pages)
        .map(|i| format!("{} 0 R", 3 + i))
        .collect();
    objects.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        n_pages
    ));
    // page objects
    let mut page_obj_ids = Vec::new();
    for i in 0..n_pages {
        let content_id = first_content + i;
        page_obj_ids.push(3 + i);
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {} 0 R >> >> /Contents {} 0 R >>",
            font_obj, content_id
        ));
    }
    // font
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());
    // content streams
    let mut content_obj_ids = Vec::new();
    for (i, page_lines) in pages.iter().enumerate() {
        content_obj_ids.push(first_content + i);
        let mut stream = String::from("BT /F1 9 Tf 50 800 Td\n");
        for (li, l) in page_lines.iter().enumerate() {
            if li == 0 {
                stream.push_str(&format!("({}) Tj\n", pdf_escape(&ascii_only(l))));
            } else {
                stream.push_str(&format!("0 -14 Td ({}) Tj\n", pdf_escape(&ascii_only(l))));
            }
        }
        stream.push_str("ET");
        objects.push(format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream));
    }

    // 序列化并写 xref
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (idx, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", idx + 1).as_bytes());
        out.extend_from_slice(obj.as_bytes());
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    let total = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {}\n", total).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", total, xref_pos).as_bytes());
    out
}

fn pdf_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

fn ascii_only(s: &str) -> String {
    s.chars().map(|c| if (c as u32) < 0x80 { c } else { '?' }).collect()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_time(ts_ms: u64) -> String {
    // 仅做粗略本地时间格式化（基于 UTC 偏移不安全，这里用毫秒直出 + 日期近似）
    let secs = ts_ms / 1000;
    let day = secs / 86400;
    let rem = secs % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    // 以 1970-01-01 为基准的天数换算日期
    let d = day + 719163; // 1970-01-01 对应的 JDN 近似
    let year = (d / 365) as u32;
    let _ = year;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ts_ms % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_contains_entries() {
        clear_for_test();
        crate::audit::ok("connect", "simbus");
        let html = gen_html("测试报表", &crate::audit::log());
        assert!(html.contains("测试报表"));
        assert!(html.contains("connect"));
    }

    #[test]
    fn test_pdf_valid_header() {
        clear_for_test();
        crate::audit::ok("flash", "fw.bin");
        let pdf = build_pdf(&["line1".to_string(), "line2 中文test".to_string()]);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        // 中文被替换为 ?
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.contains("line2 ??test"));
    }

    fn clear_for_test() {
        crate::audit::clear();
    }
}
