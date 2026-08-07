#!/bin/bash
cd /d/proj/tx_di
cargo test -p admin_app -- --test-threads=1 > /tmp/admin_app.log 2>&1
echo "EXIT=$?"
grep -E "Running|test result|error\[" /tmp/admin_app.log | head -60
