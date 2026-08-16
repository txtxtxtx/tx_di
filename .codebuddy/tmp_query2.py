import sqlite3

conn = sqlite3.connect('examples/tx_admin/data/tx_admin.db')
cur = conn.cursor()

print('=== 登录日志最新5条 ===')
cur.execute("SELECT id, user_id, username, login_ip, login_type, result, msg FROM sys_login_log ORDER BY id DESC LIMIT 5")
for r in cur.fetchall():
    print(r)

print()
print('=== 操作日志最新5条（含request字段）===')
cur.execute("SELECT id, trace_id, user_id, user_type, log_type, sub_type, biz_id, action, success, request_method, request_url, user_ip, user_agent FROM sys_operate_log ORDER BY id DESC LIMIT 5")
for r in cur.fetchall():
    print(r)

conn.close()
