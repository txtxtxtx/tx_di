#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""校验 openapi.yaml / openapi_v3.yaml 的合法性与引用完整性。"""
import yaml
import os

HERE = os.path.dirname(os.path.abspath(__file__))
paths_file = os.path.join(HERE, "openapi.yaml")
v3_file = os.path.join(HERE, "openapi_v3.yaml")

with open(paths_file, encoding="utf-8") as f:
    d = yaml.safe_load(f)

paths = d["paths"]
schemas = d["components"]["schemas"]
print("paths:", len(paths), "schemas:", len(schemas))

refs = set()


def walk(o):
    if isinstance(o, dict):
        for k, v in o.items():
            if k == "$ref" and isinstance(v, str):
                refs.add(v)
            else:
                walk(v)
    elif isinstance(o, list):
        for i in o:
            walk(i)


walk(d)
miss = [r for r in refs if r.startswith("#/components/schemas/") and r.split("/")[-1] not in schemas]
print("missing schema refs:", miss)
print("has /health:", "/health" in paths, "| has upload:", "/api/v1/file/upload" in paths)
print(
    "v3 identical to v3 file:",
    open(paths_file, encoding="utf-8").read() == open(v3_file, encoding="utf-8").read(),
)
print("yaml size (bytes):", os.path.getsize(paths_file))
