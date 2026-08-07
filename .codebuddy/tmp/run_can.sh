#!/bin/bash
cd /d/proj/tx_di
cargo test -p tx_di_can > /tmp/can.log 2>&1
echo "EXIT=$?"
grep -E "Running|test result|error\[" /tmp/can.log | head -30
