#!/bin/bash
cd /d/proj/tx_di
cargo test -p admin_domain -p admin_infra -p admin_macros -- --test-threads=1 > /tmp/admin1.log 2>&1
echo "EXIT=$?"
grep -E "Running|test result|error\[" /tmp/admin1.log | head -60
