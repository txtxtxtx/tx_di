import sqlite3

conn = sqlite3.connect('examples/tx_admin/data/tx_admin.db')
cur = conn.cursor()

print('=== 操作日志 auth 相关 ===')
cur.execute("SELECT id, trace_id, user_id, log_type, sub_type, action, success, request_method, request_url, user_ip FROM sys_operate_log WHERE action LIKE '%auth%' ORDER BY id")
for r in cur.fetchall():
    print(r)

print()
print('=== 操作日志 success=0 ===')
cur.execute("SELECT id, trace_id, user_id, sub_type, action, success FROM sys_operate_log WHERE success=0 ORDER BY id")
for r in cur.fetchall():
    print(r)

print()
print('=== 操作日志总数 ===')
print(cur.execute('SELECT COUNT(*) FROM sys_operate_log').fetchone()[0])
print('=== 登录日志总数 ===')
print(cur.execute('SELECT COUNT(*) FROM sys_login_log').fetchone()[0])
print('=== 操作日志最新10条 ===')
cur.execute("SELECT id, trace_id, user_id, log_type, sub_type, action, success FROM sys_operate_log ORDER BY id DESC LIMIT 10")
for r in cur.fetchall():
    print(r)

conn.close()
