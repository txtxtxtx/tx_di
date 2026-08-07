#!/bin/bash
grep -E "Compiling|Running|Doc-tests|test result|error\[" /tmp/admin_test.log | head -60
