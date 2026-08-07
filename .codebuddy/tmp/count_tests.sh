#!/bin/bash
cd /d/proj/tx_di
echo "===== admin_app tests ====="
for f in examples/tx_admin/admin_app/tests/*_test.rs; do
  n=$(grep -cE '^\s*#\[(tokio::)?test' "$f")
  echo "$(basename "$f"): $n"
done
echo "===== integration_test.rs size ====="
wc -c examples/tx_admin/admin_app/tests/integration_test.rs
echo "===== common/mod.rs head ====="
head -50 examples/tx_admin/admin_app/tests/common/mod.rs
echo "===== admin_api tests ====="
for f in examples/tx_admin/admin_api/tests/*_test.rs; do
  n=$(grep -cE '^\s*#\[(tokio::)?test' "$f")
  echo "$(basename "$f"): $n"
done
echo "===== tx-di-core tests ====="
for f in tx-di-core/tests/*.rs; do
  n=$(grep -cE '^\s*#\[(tokio::)?test' "$f")
  echo "$(basename "$f"): $n"
done
echo "===== domain tests ====="
find examples/tx_admin/admin_domain/src -name tests.rs | while read -r f; do
  n=$(grep -cE '^\s*#\[(tokio::)?test' "$f")
  echo "$f: $n"
done
echo "===== tx_di_can tests ====="
grep -cE '^\s*#\[(tokio::)?test' examples/tx_di_can/src/tests.rs 2>/dev/null
grep -cE '^\s*#\[(tokio::)?test' examples/tx_di_can/src/sim_ecu/tests.rs 2>/dev/null
