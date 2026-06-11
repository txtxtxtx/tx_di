#!/usr/bin/env python3
"""
analyze_rust_ddd.py
-------------------
Analyze a Rust DDD project structure and generate a test coverage report
showing which items need tests and suggesting test file locations.

Usage:
    python analyze_rust_ddd.py <project_root>

Output:
    - Console summary of public items per layer
    - Suggested test file structure
    - Missing test checklist
"""

import os
import sys
import re
from pathlib import Path
from collections import defaultdict

# ─── Patterns ────────────────────────────────────────────────────────────────
PUB_FN    = re.compile(r'^\s*pub(?:\s+async)?\s+fn\s+(\w+)')
PUB_STRUCT = re.compile(r'^\s*pub\s+struct\s+(\w+)')
PUB_ENUM  = re.compile(r'^\s*pub\s+enum\s+(\w+)')
PUB_TRAIT = re.compile(r'^\s*pub\s+trait\s+(\w+)')
IMPL_BLOCK = re.compile(r'^\s*impl(?:<[^>]*>)?\s+(\w+)')
TEST_ATTR = re.compile(r'#\[(?:tokio::)?test\]')
CFG_TEST  = re.compile(r'#\[cfg\(test\)\]')

# DDD layer detection by directory name patterns
DDD_LAYERS = {
    'domain':         'Domain',
    'application':    'Application',
    'infrastructure': 'Infrastructure',
    'infra':          'Infrastructure',
    'interfaces':     'Interface',
    'interface':      'Interface',
    'api':            'Interface',
    'handlers':       'Interface',
    'adapters':       'Infrastructure',
}


def detect_layer(path: Path) -> str:
    parts = [p.lower() for p in path.parts]
    for part in parts:
        if part in DDD_LAYERS:
            return DDD_LAYERS[part]
    return 'Unknown'


def analyze_file(filepath: Path) -> dict:
    """Analyze a single .rs file and return its public items and test status."""
    result = {
        'pub_fns': [],
        'pub_structs': [],
        'pub_enums': [],
        'pub_traits': [],
        'impl_blocks': [],
        'has_tests': False,
    }
    try:
        content = filepath.read_text(encoding='utf-8', errors='ignore')
        lines = content.splitlines()
        for line in lines:
            m = PUB_FN.match(line)
            if m and m.group(1) not in ('new', 'default'):
                result['pub_fns'].append(m.group(1))
            m = PUB_STRUCT.match(line)
            if m:
                result['pub_structs'].append(m.group(1))
            m = PUB_ENUM.match(line)
            if m:
                result['pub_enums'].append(m.group(1))
            m = PUB_TRAIT.match(line)
            if m:
                result['pub_traits'].append(m.group(1))
            m = IMPL_BLOCK.match(line)
            if m:
                result['impl_blocks'].append(m.group(1))
        result['has_tests'] = bool(TEST_ATTR.search(content) or CFG_TEST.search(content))
    except Exception as e:
        pass
    return result


def find_rust_files(root: Path) -> list:
    files = []
    for f in root.rglob('*.rs'):
        # Skip generated files and target directory
        if 'target' in f.parts or 'generated' in f.parts:
            continue
        files.append(f)
    return sorted(files)


def print_section(title: str, char: str = '─', width: int = 65):
    print(f"\n{char * width}")
    print(f"  {title}")
    print(f"{char * width}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_rust_ddd.py <project_root>")
        sys.exit(1)

    root = Path(sys.argv[1]).resolve()
    if not root.exists():
        print(f"Error: {root} does not exist")
        sys.exit(1)

    print(f"\n🔍  Analyzing Rust DDD project: {root}")

    rs_files = find_rust_files(root / 'src')
    if not rs_files:
        rs_files = find_rust_files(root)

    by_layer = defaultdict(list)
    for f in rs_files:
        layer = detect_layer(f.relative_to(root))
        analysis = analyze_file(f)
        analysis['path'] = f.relative_to(root)
        by_layer[layer].append(analysis)

    # ── Summary ────────────────────────────────────────────────────────────
    print_section("PROJECT SUMMARY", '═')
    total_files = len(rs_files)
    files_with_tests = sum(1 for items in by_layer.values() for a in items if a['has_tests'])
    print(f"  Total .rs files : {total_files}")
    print(f"  Files with tests: {files_with_tests}  ({100*files_with_tests//max(total_files,1)}%)")

    # ── Per-layer breakdown ────────────────────────────────────────────────
    checklist = []
    for layer in ['Domain', 'Application', 'Infrastructure', 'Interface', 'Unknown']:
        items = by_layer.get(layer, [])
        if not items:
            continue
        print_section(f"{layer.upper()} LAYER  ({len(items)} files)")

        for a in items:
            p = a['path']
            tested = "✅" if a['has_tests'] else "❌"
            print(f"\n  {tested} {p}")
            if a['pub_structs']:
                print(f"     Structs  : {', '.join(a['pub_structs'])}")
            if a['pub_enums']:
                print(f"     Enums    : {', '.join(a['pub_enums'])}")
            if a['pub_traits']:
                print(f"     Traits   : {', '.join(a['pub_traits'])}")
            if a['pub_fns']:
                print(f"     Pub fns  : {', '.join(a['pub_fns'])}")

            if not a['has_tests'] and (a['pub_fns'] or a['pub_structs']):
                checklist.append((layer, str(p), a))

    # ── Missing tests checklist ─────────────────────────────────────────────
    if checklist:
        print_section("⚠️  FILES MISSING TESTS", '─')
        for layer, path, a in checklist:
            items = a['pub_fns'] + a['pub_structs'] + a['pub_traits']
            print(f"  [{layer}] {path}")
            for item in items[:5]:
                print(f"    → test_{item.lower()}_*")
            if len(items) > 5:
                print(f"    → ... and {len(items) - 5} more")

    # ── Suggested test file structure ────────────────────────────────────────
    print_section("📁  SUGGESTED TEST STRUCTURE", '─')
    suggestions = [
        "src/domain/tests/mod.rs        -- Domain unit tests",
        "src/application/tests/mod.rs   -- Application unit tests (mocked repos)",
        "tests/integration/             -- Integration tests (real DB/adapters)",
        "tests/e2e/                     -- End-to-end tests (full app stack)",
    ]
    for s in suggestions:
        print(f"  {s}")

    # ── Dev-dependencies reminder ────────────────────────────────────────────
    print_section("📦  RECOMMENDED DEV-DEPENDENCIES (Cargo.toml)", '─')
    deps = [
        'tokio       = { version = "1", features = ["full", "test-util"] }',
        'mockall     = "0.13"',
        'fake        = { version = "2", features = ["derive"] }',
        'rstest      = "0.23"',
        'pretty_assertions = "1"',
        'insta       = "1"',
        'proptest    = "1"',
        'wiremock    = "0.6"',
    ]
    print("\n  [dev-dependencies]")
    for d in deps:
        print(f"  {d}")

    print(f"\n✅  Analysis complete. {len(checklist)} file(s) need tests.\n")


if __name__ == '__main__':
    main()
