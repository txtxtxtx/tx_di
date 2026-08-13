#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""零依赖合并脚本：将 openapi_base.yaml + parts/*.yaml 拼合为 openapi.yaml / openapi_v3.yaml。

每个 part 文件必须包含两节（顺序任意）：
  # ===== PATHS =====    其下为 paths 的子条目（2 空格缩进的路径块）
  # ===== SCHEMAS =====  其下为 components.schemas 的子条目（4 空格缩进）
合并时：paths 块插入到 base 中 `paths:` 之后；schemas 块插入到 `  schemas:` 之后。
"""
import glob
import os

HERE = os.path.dirname(os.path.abspath(__file__))
BASE = os.path.join(HERE, "openapi_base.yaml")
PARTS_DIR = os.path.join(HERE, "parts")
OUT_YAML = os.path.join(HERE, "openapi.yaml")
OUT_V3 = os.path.join(HERE, "openapi_v3.yaml")


def collect():
    paths_lines = []
    schemas_lines = []
    for pf in sorted(glob.glob(os.path.join(PARTS_DIR, "*.yaml"))):
        with open(pf, "r", encoding="utf-8") as f:
            lines = f.read().split("\n")
        mode = None
        for ln in lines:
            s = ln.strip()
            if s == "# ===== PATHS =====":
                mode = "paths"
                continue
            if s == "# ===== SCHEMAS =====":
                mode = "schemas"
                continue
            if mode == "paths":
                paths_lines.append(ln)
            elif mode == "schemas":
                schemas_lines.append(ln)
    return paths_lines, schemas_lines


def count_entries(lines, prefix):
    return sum(1 for l in lines if l.startswith(prefix) and ":" in l and not l.strip().startswith("#"))


def merge():
    paths_lines, schemas_lines = collect()
    with open(BASE, "r", encoding="utf-8") as f:
        base = f.read().split("\n")

    out = []
    for ln in base:
        out.append(ln)
        if ln.strip() == "paths:":
            out.extend(paths_lines)
        if ln.rstrip() == "  schemas:":
            out.extend(schemas_lines)

    content = "\n".join(out)
    with open(OUT_YAML, "w", encoding="utf-8") as f:
        f.write(content)
    with open(OUT_V3, "w", encoding="utf-8") as f:
        f.write(content)

    print("merged parts:", len(sorted(glob.glob(os.path.join(PARTS_DIR, "*.yaml")))))
    print("path entries:", count_entries(paths_lines, "  /"))
    print("schema entries:", count_entries(schemas_lines, "    "))


if __name__ == "__main__":
    merge()
