import sqlite3

conn = sqlite3.connect('examples/tx_admin/data/tx_admin.db')
cur = conn.cursor()

print('=== sys_operate_result 字典数据 ===')
cur.execute("SELECT id, label, value, color_type FROM sys_dict_data WHERE dict_type='sys_operate_result'")
for r in cur.fetchall():
    print(r)

print()
print('=== 登录日志 result 分布 ===')
cur.execute("SELECT result, COUNT(*) FROM sys_login_log GROUP BY result")
for r in cur.fetchall():
    print(r)

print()
print('=== 操作日志 success 分布 ===')
cur.execute("SELECT success, COUNT(*) FROM sys_operate_log GROUP BY success")
for r in cur.fetchall():
    print(r)

conn.close()
