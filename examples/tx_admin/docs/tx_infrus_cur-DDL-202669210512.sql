DROP TABLE IF EXISTS system_config;
CREATE TABLE system_config(
    id BIGINT NOT NULL,
    category VARCHAR(100) NOT NULL,
    type INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    config_key VARCHAR(100) NOT NULL,
    value VARCHAR(900) NOT NULL,
    visible INTEGER NOT NULL,
    remark VARCHAR(200),
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_config IS '参数配置表';
COMMENT ON COLUMN system_config.id IS '参数主键';
COMMENT ON COLUMN system_config.category IS '参数分组';
COMMENT ON COLUMN system_config.type IS '参数类型';
COMMENT ON COLUMN system_config.name IS '参数名称';
COMMENT ON COLUMN system_config.config_key IS '参数键名';
COMMENT ON COLUMN system_config.value IS '参数键值';
COMMENT ON COLUMN system_config.visible IS '是否可见';
COMMENT ON COLUMN system_config.remark IS '备注';
COMMENT ON COLUMN system_config.creator IS '创建者';
COMMENT ON COLUMN system_config.create_time IS '创建时间';
COMMENT ON COLUMN system_config.updater IS '更新者';
COMMENT ON COLUMN system_config.update_time IS '更新时间';
COMMENT ON COLUMN system_config.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_infra_config ON system_config(id);

DROP TABLE IF EXISTS infrust_file;
CREATE TABLE infrust_file(
    id BIGINT NOT NULL,
    config_id INTEGER,
    name VARCHAR(200) NOT NULL,
    path VARCHAR(900) NOT NULL,
    url VARCHAR(900) NOT NULL,
    type VARCHAR(100),
    size INTEGER NOT NULL,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE infrust_file IS '文件表';
COMMENT ON COLUMN infrust_file.id IS '文件编号';
COMMENT ON COLUMN infrust_file.config_id IS '配置编号';
COMMENT ON COLUMN infrust_file.name IS '文件名';
COMMENT ON COLUMN infrust_file.path IS '文件路径';
COMMENT ON COLUMN infrust_file.url IS '文件 URL';
COMMENT ON COLUMN infrust_file.type IS '文件类型';
COMMENT ON COLUMN infrust_file.size IS '文件大小（B 字节）';
COMMENT ON COLUMN infrust_file.creator IS '创建者';
COMMENT ON COLUMN infrust_file.create_time IS '创建时间';
COMMENT ON COLUMN infrust_file.updater IS '更新者';
COMMENT ON COLUMN infrust_file.update_time IS '更新时间';
COMMENT ON COLUMN infrust_file.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_infra_file ON infrust_file(id);

DROP TABLE IF EXISTS infrust_file_config;
CREATE TABLE infrust_file_config(
    id INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    storage INTEGER NOT NULL,
    remark VARCHAR(200),
    master INTEGER NOT NULL DEFAULT  0,
    config VARCHAR(900) NOT NULL,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE infrust_file_config IS '文件配置表';
COMMENT ON COLUMN infrust_file_config.id IS '编号';
COMMENT ON COLUMN infrust_file_config.name IS '配置名';
COMMENT ON COLUMN infrust_file_config.storage IS '存储器';
COMMENT ON COLUMN infrust_file_config.remark IS '备注';
COMMENT ON COLUMN infrust_file_config.master IS '是否为主配置(0否 1是)';
COMMENT ON COLUMN infrust_file_config.config IS '存储配置';
COMMENT ON COLUMN infrust_file_config.creator IS '创建者';
COMMENT ON COLUMN infrust_file_config.create_time IS '创建时间';
COMMENT ON COLUMN infrust_file_config.updater IS '更新者';
COMMENT ON COLUMN infrust_file_config.update_time IS '更新时间';
COMMENT ON COLUMN infrust_file_config.deleted IS '是否删除（0 否 1是）';


CREATE UNIQUE INDEX pk_infra_file_config ON infrust_file_config(id);

DROP TABLE IF EXISTS infrust_file_content;
CREATE TABLE infrust_file_content(
    id BIGINT NOT NULL,
    config_id INTEGER NOT NULL,
    path VARCHAR(900) NOT NULL,
    content BYTEA NOT NULL,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE infrust_file_content IS '文件存储表';
COMMENT ON COLUMN infrust_file_content.id IS '编号';
COMMENT ON COLUMN infrust_file_content.config_id IS '配置编号';
COMMENT ON COLUMN infrust_file_content.path IS '文件路径';
COMMENT ON COLUMN infrust_file_content.content IS '文件内容';
COMMENT ON COLUMN infrust_file_content.creator IS '创建者';
COMMENT ON COLUMN infrust_file_content.create_time IS '创建时间';
COMMENT ON COLUMN infrust_file_content.updater IS '更新者';
COMMENT ON COLUMN infrust_file_content.update_time IS '更新时间';
COMMENT ON COLUMN infrust_file_content.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_infra_file_content ON infrust_file_content(id);

DROP TABLE IF EXISTS infrust_job;
CREATE TABLE infrust_job(
    id BIGINT NOT NULL,
    name VARCHAR(30) NOT NULL,
    status INTEGER NOT NULL,
    handler_name VARCHAR(200) NOT NULL,
    handler_param VARCHAR(900),
    cron_expression VARCHAR(30) NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT  0,
    retry_interval INTEGER NOT NULL DEFAULT  0,
    monitor_timeout INTEGER NOT NULL DEFAULT  0,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL,
    PRIMARY KEY (id)
);

COMMENT ON TABLE infrust_job IS '定时任务表';
COMMENT ON COLUMN infrust_job.id IS '任务编号';
COMMENT ON COLUMN infrust_job.name IS '任务名称';
COMMENT ON COLUMN infrust_job.status IS '任务状态';
COMMENT ON COLUMN infrust_job.handler_name IS '处理器的名字或url';
COMMENT ON COLUMN infrust_job.handler_param IS '处理器的参数(post json)';
COMMENT ON COLUMN infrust_job.cron_expression IS 'CRON 表达式';
COMMENT ON COLUMN infrust_job.retry_count IS '重试次数';
COMMENT ON COLUMN infrust_job.retry_interval IS '重试间隔(单位 s)';
COMMENT ON COLUMN infrust_job.monitor_timeout IS '监控超时时间(单位 s)';
COMMENT ON COLUMN infrust_job.creator IS '创建者';
COMMENT ON COLUMN infrust_job.create_time IS '创建时间';
COMMENT ON COLUMN infrust_job.updater IS '更新者';
COMMENT ON COLUMN infrust_job.update_time IS '更新时间';
COMMENT ON COLUMN infrust_job.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_infra_job ON infrust_job(id);

DROP TABLE IF EXISTS infrust_job_log;
CREATE TABLE infrust_job_log(
    id int8 NOT NULL,
    job_id int8 NOT NULL,
    handler_name VARCHAR(64) NOT NULL,
    handler_param VARCHAR(30) DEFAULT  'NULL::character varying',
    execute_index int2 NOT NULL DEFAULT  1,
    begin_time timestamp NOT NULL,
    end_time timestamp,
    duration int4,
    status int2 NOT NULL,
    result VARCHAR(4000) DEFAULT  '',
    creator VARCHAR(64) DEFAULT  '',
    create_time timestamp NOT NULL DEFAULT  CURRENT_TIMESTAMP,
    updater VARCHAR(64) DEFAULT  '',
    update_time timestamp NOT NULL DEFAULT  CURRENT_TIMESTAMP,
    deleted int2 NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE infrust_job_log IS '定时任务日志表';
COMMENT ON COLUMN infrust_job_log.id IS '日志编号';
COMMENT ON COLUMN infrust_job_log.job_id IS '任务编号';
COMMENT ON COLUMN infrust_job_log.handler_name IS '处理器的名字';
COMMENT ON COLUMN infrust_job_log.handler_param IS '处理器的参数';
COMMENT ON COLUMN infrust_job_log.execute_index IS '第几次执行';
COMMENT ON COLUMN infrust_job_log.begin_time IS '开始执行时间';
COMMENT ON COLUMN infrust_job_log.end_time IS '结束执行时间';
COMMENT ON COLUMN infrust_job_log.duration IS '执行时长';
COMMENT ON COLUMN infrust_job_log.status IS '任务状态';
COMMENT ON COLUMN infrust_job_log.result IS '结果数据';
COMMENT ON COLUMN infrust_job_log.creator IS '创建者';
COMMENT ON COLUMN infrust_job_log.create_time IS '创建时间';
COMMENT ON COLUMN infrust_job_log.updater IS '更新者';
COMMENT ON COLUMN infrust_job_log.update_time IS '更新时间';
COMMENT ON COLUMN infrust_job_log.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_infra_job_log ON infrust_job_log(id);

DROP TABLE IF EXISTS system_dept;
CREATE TABLE system_dept(
    id INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    parent_id INTEGER NOT NULL DEFAULT  0,
    sort INTEGER NOT NULL,
    leader_user_id BIGINT,
    phone VARCHAR(30),
    email VARCHAR(100),
    status INTEGER NOT NULL,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    tenant_id INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_dept IS '部门表';
COMMENT ON COLUMN system_dept.id IS '部门id';
COMMENT ON COLUMN system_dept.name IS '部门名称';
COMMENT ON COLUMN system_dept.parent_id IS '父部门id';
COMMENT ON COLUMN system_dept.sort IS '显示顺序';
COMMENT ON COLUMN system_dept.leader_user_id IS '负责人';
COMMENT ON COLUMN system_dept.phone IS '联系电话';
COMMENT ON COLUMN system_dept.email IS '邮箱';
COMMENT ON COLUMN system_dept.status IS '部门状态（0正常 1停用）';
COMMENT ON COLUMN system_dept.creator IS '创建者';
COMMENT ON COLUMN system_dept.create_time IS '创建时间';
COMMENT ON COLUMN system_dept.updater IS '更新者';
COMMENT ON COLUMN system_dept.update_time IS '更新时间';
COMMENT ON COLUMN system_dept.deleted IS '是否删除';
COMMENT ON COLUMN system_dept.tenant_id IS '租户编号';

DROP TABLE IF EXISTS system_dict_data;
CREATE TABLE system_dict_data(
    id BIGINT NOT NULL,
    sort INTEGER NOT NULL DEFAULT  0,
    label VARCHAR(100) NOT NULL,
    value VARCHAR(100) NOT NULL,
    dict_type VARCHAR(100) NOT NULL,
    status INTEGER NOT NULL DEFAULT  0,
    color_type VARCHAR(100),
    css_class VARCHAR(100),
    remark VARCHAR(500),
    creator VARCHAR(64),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(64),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_dict_data IS '字典数据表';
COMMENT ON COLUMN system_dict_data.id IS '字典编码';
COMMENT ON COLUMN system_dict_data.sort IS '字典排序';
COMMENT ON COLUMN system_dict_data.label IS '字典标签';
COMMENT ON COLUMN system_dict_data.value IS '字典键值';
COMMENT ON COLUMN system_dict_data.dict_type IS '字典类型';
COMMENT ON COLUMN system_dict_data.status IS '状态（0正常 1停用）';
COMMENT ON COLUMN system_dict_data.color_type IS '颜色类型';
COMMENT ON COLUMN system_dict_data.css_class IS 'css 样式';
COMMENT ON COLUMN system_dict_data.remark IS '备注';
COMMENT ON COLUMN system_dict_data.creator IS '创建者';
COMMENT ON COLUMN system_dict_data.create_time IS '创建时间';
COMMENT ON COLUMN system_dict_data.updater IS '更新者';
COMMENT ON COLUMN system_dict_data.update_time IS '更新时间';
COMMENT ON COLUMN system_dict_data.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_system_dict_data ON system_dict_data(id);

DROP TABLE IF EXISTS system_dict_type;
CREATE TABLE system_dict_type(
    id BIGINT NOT NULL,
    name VARCHAR(100) NOT NULL,
    type VARCHAR(100) NOT NULL,
    status INTEGER NOT NULL DEFAULT  0,
    remark VARCHAR(500),
    creator VARCHAR(64),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(64),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    deleted_time TIMESTAMPTZ,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_dict_type IS '字典类型表';
COMMENT ON COLUMN system_dict_type.id IS '字典主键';
COMMENT ON COLUMN system_dict_type.name IS '字典名称';
COMMENT ON COLUMN system_dict_type.type IS '字典类型';
COMMENT ON COLUMN system_dict_type.status IS '状态（0正常 1停用）';
COMMENT ON COLUMN system_dict_type.remark IS '备注';
COMMENT ON COLUMN system_dict_type.creator IS '创建者';
COMMENT ON COLUMN system_dict_type.create_time IS '创建时间';
COMMENT ON COLUMN system_dict_type.updater IS '更新者';
COMMENT ON COLUMN system_dict_type.update_time IS '更新时间';
COMMENT ON COLUMN system_dict_type.deleted IS '是否删除';
COMMENT ON COLUMN system_dict_type.deleted_time IS '删除时间';


CREATE UNIQUE INDEX pk_system_dict_type ON system_dict_type(id);

DROP TABLE IF EXISTS system_menu;
CREATE TABLE system_menu(
    id INTEGER NOT NULL,
    name VARCHAR(30) NOT NULL,
    permission VARCHAR(100) NOT NULL,
    types INTEGER NOT NULL,
    sort INTEGER NOT NULL,
    parent_id INTEGER NOT NULL DEFAULT  0,
    path VARCHAR(200),
    icon VARCHAR(200),
    component VARCHAR(100),
    component_name VARCHAR(30),
    status INTEGER NOT NULL DEFAULT  0,
    visible INTEGER NOT NULL DEFAULT  0,
    keep_alive INTEGER NOT NULL DEFAULT  0,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    tenant_id INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_menu IS '菜单权限表';
COMMENT ON COLUMN system_menu.id IS '菜单ID';
COMMENT ON COLUMN system_menu.name IS '菜单名称';
COMMENT ON COLUMN system_menu.permission IS '权限标识';
COMMENT ON COLUMN system_menu.types IS '菜单类型';
COMMENT ON COLUMN system_menu.sort IS '显示顺序';
COMMENT ON COLUMN system_menu.parent_id IS '父菜单ID';
COMMENT ON COLUMN system_menu.path IS '路由地址';
COMMENT ON COLUMN system_menu.icon IS '菜单图标';
COMMENT ON COLUMN system_menu.component IS '组件路径';
COMMENT ON COLUMN system_menu.component_name IS '组件名';
COMMENT ON COLUMN system_menu.status IS '菜单状态';
COMMENT ON COLUMN system_menu.visible IS '是否可见';
COMMENT ON COLUMN system_menu.keep_alive IS '是否缓存';
COMMENT ON COLUMN system_menu.creator IS '创建者';
COMMENT ON COLUMN system_menu.create_time IS '创建时间';
COMMENT ON COLUMN system_menu.updater IS '更新者';
COMMENT ON COLUMN system_menu.update_time IS '更新时间';
COMMENT ON COLUMN system_menu.deleted IS '是否删除';
COMMENT ON COLUMN system_menu.tenant_id IS '租户编号（默认 0）';

DROP TABLE IF EXISTS system_operate_log;
CREATE TABLE system_operate_log(
    id BIGINT NOT NULL,
    trace_id VARCHAR(100) NOT NULL,
    user_id BIGINT NOT NULL,
    user_type INTEGER NOT NULL DEFAULT  0,
    type VARCHAR(100) NOT NULL,
    sub_type VARCHAR(100) NOT NULL,
    biz_id BIGINT NOT NULL,
    action VARCHAR(900) NOT NULL,
    success INTEGER NOT NULL,
    extra VARCHAR(900) NOT NULL,
    request_method VARCHAR(200),
    request_url VARCHAR(200),
    user_ip VARCHAR(30),
    user_agent VARCHAR(900),
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    tenant_id INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_operate_log IS '操作日志记录';
COMMENT ON COLUMN system_operate_log.id IS '日志主键';
COMMENT ON COLUMN system_operate_log.trace_id IS '链路追踪编号';
COMMENT ON COLUMN system_operate_log.user_id IS '用户编号';
COMMENT ON COLUMN system_operate_log.user_type IS '用户类型';
COMMENT ON COLUMN system_operate_log.type IS '操作模块类型';
COMMENT ON COLUMN system_operate_log.sub_type IS '操作名';
COMMENT ON COLUMN system_operate_log.biz_id IS '操作数据模块编号';
COMMENT ON COLUMN system_operate_log.action IS '操作内容';
COMMENT ON COLUMN system_operate_log.success IS '操作结果';
COMMENT ON COLUMN system_operate_log.extra IS '拓展字段';
COMMENT ON COLUMN system_operate_log.request_method IS '请求方法名';
COMMENT ON COLUMN system_operate_log.request_url IS '请求地址';
COMMENT ON COLUMN system_operate_log.user_ip IS '用户 IP';
COMMENT ON COLUMN system_operate_log.user_agent IS '浏览器 UA';
COMMENT ON COLUMN system_operate_log.creator IS '创建者';
COMMENT ON COLUMN system_operate_log.create_time IS '创建时间';
COMMENT ON COLUMN system_operate_log.updater IS '更新者';
COMMENT ON COLUMN system_operate_log.update_time IS '更新时间';
COMMENT ON COLUMN system_operate_log.deleted IS '是否删除';
COMMENT ON COLUMN system_operate_log.tenant_id IS '租户编号';

DROP TABLE IF EXISTS system_role;
CREATE TABLE system_role(
    id INTEGER NOT NULL,
    name VARCHAR(30) NOT NULL,
    code VARCHAR(100) NOT NULL,
    sort INTEGER NOT NULL,
    data_scope INTEGER NOT NULL DEFAULT  4,
    data_scope_dept_ids VARCHAR(900),
    status INTEGER NOT NULL,
    remark VARCHAR(200),
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    tenant_id INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_role IS '角色表';
COMMENT ON COLUMN system_role.id IS '角色ID(10000-99999)';
COMMENT ON COLUMN system_role.name IS '角色名称';
COMMENT ON COLUMN system_role.code IS '角色权限字符串';
COMMENT ON COLUMN system_role.sort IS '显示顺序';
COMMENT ON COLUMN system_role.data_scope IS '数据范围（1全部数据权限 2自定数据权限 3本部门数据权限 4本部门及以下数据权限）';
COMMENT ON COLUMN system_role.data_scope_dept_ids IS '数据范围 ( 指定部门数组 )';
COMMENT ON COLUMN system_role.status IS '角色状态（0正常 1停用）';
COMMENT ON COLUMN system_role.remark IS '备注';
COMMENT ON COLUMN system_role.creator IS '创建者';
COMMENT ON COLUMN system_role.create_time IS '创建时间';
COMMENT ON COLUMN system_role.updater IS '更新者';
COMMENT ON COLUMN system_role.update_time IS '更新时间';
COMMENT ON COLUMN system_role.deleted IS '是否删除';
COMMENT ON COLUMN system_role.tenant_id IS '租户编号';

DROP TABLE IF EXISTS system_role_menu;
CREATE TABLE system_role_menu(
    role_id INTEGER NOT NULL,
    menu_id INTEGER NOT NULL,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (role_id,menu_id)
);

COMMENT ON TABLE system_role_menu IS '角色和菜单关联表';
COMMENT ON COLUMN system_role_menu.role_id IS '角色ID';
COMMENT ON COLUMN system_role_menu.menu_id IS '菜单ID';
COMMENT ON COLUMN system_role_menu.creator IS '创建者';
COMMENT ON COLUMN system_role_menu.create_time IS '创建时间';
COMMENT ON COLUMN system_role_menu.updater IS '更新者';
COMMENT ON COLUMN system_role_menu.update_time IS '更新时间';
COMMENT ON COLUMN system_role_menu.deleted IS '是否删除';

DROP TABLE IF EXISTS system_tenant;
CREATE TABLE system_tenant(
    id BIGINT NOT NULL,
    name VARCHAR(30) NOT NULL,
    contact_user_id BIGINT,
    contact_name VARCHAR(30) NOT NULL,
    contact_mobile VARCHAR(500) DEFAULT  'NULL::character varying',
    status INTEGER NOT NULL DEFAULT  0,
    websites VARCHAR(1024) DEFAULT  '',
    package_id BIGINT NOT NULL,
    expire_time TIMESTAMPTZ NOT NULL,
    account_count INTEGER NOT NULL,
    creator VARCHAR(64) NOT NULL DEFAULT  '',
    create_time TIMESTAMPTZ NOT NULL DEFAULT  CURRENT_TIMESTAMP,
    updater VARCHAR(64) DEFAULT  '',
    update_time TIMESTAMPTZ NOT NULL DEFAULT  CURRENT_TIMESTAMP,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_tenant IS '租户表';
COMMENT ON COLUMN system_tenant.id IS '租户编号';
COMMENT ON COLUMN system_tenant.name IS '租户名';
COMMENT ON COLUMN system_tenant.contact_user_id IS '联系人的用户编号';
COMMENT ON COLUMN system_tenant.contact_name IS '联系人';
COMMENT ON COLUMN system_tenant.contact_mobile IS '联系手机';
COMMENT ON COLUMN system_tenant.status IS '租户状态';
COMMENT ON COLUMN system_tenant.websites IS '绑定域名数组';
COMMENT ON COLUMN system_tenant.package_id IS '租户套餐编号';
COMMENT ON COLUMN system_tenant.expire_time IS '过期时间';
COMMENT ON COLUMN system_tenant.account_count IS '账号数量';
COMMENT ON COLUMN system_tenant.creator IS '创建者';
COMMENT ON COLUMN system_tenant.create_time IS '创建时间';
COMMENT ON COLUMN system_tenant.updater IS '更新者';
COMMENT ON COLUMN system_tenant.update_time IS '更新时间';
COMMENT ON COLUMN system_tenant.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_system_tenant ON system_tenant(id);

DROP TABLE IF EXISTS system_tenant_package;
CREATE TABLE system_tenant_package(
    id BIGINT NOT NULL,
    name VARCHAR(30) NOT NULL,
    status INTEGER NOT NULL DEFAULT  0,
    remark VARCHAR(256) DEFAULT  '',
    menu_ids VARCHAR(4096) NOT NULL,
    creator VARCHAR(64) NOT NULL DEFAULT  '',
    create_time TIMESTAMPTZ NOT NULL DEFAULT  CURRENT_TIMESTAMP,
    updater VARCHAR(64) DEFAULT  '',
    update_time TIMESTAMPTZ NOT NULL DEFAULT  CURRENT_TIMESTAMP,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_tenant_package IS '租户套餐表';
COMMENT ON COLUMN system_tenant_package.id IS '套餐编号';
COMMENT ON COLUMN system_tenant_package.name IS '套餐名';
COMMENT ON COLUMN system_tenant_package.status IS '租户状态（0正常 1停用）';
COMMENT ON COLUMN system_tenant_package.remark IS '备注';
COMMENT ON COLUMN system_tenant_package.menu_ids IS '关联的菜单编号';
COMMENT ON COLUMN system_tenant_package.creator IS '创建者';
COMMENT ON COLUMN system_tenant_package.create_time IS '创建时间';
COMMENT ON COLUMN system_tenant_package.updater IS '更新者';
COMMENT ON COLUMN system_tenant_package.update_time IS '更新时间';
COMMENT ON COLUMN system_tenant_package.deleted IS '是否删除';


CREATE UNIQUE INDEX pk_system_tenant_package ON system_tenant_package(id);

DROP TABLE IF EXISTS system_user_role;
CREATE TABLE system_user_role(
    user_id BIGINT NOT NULL,
    role_id INTEGER NOT NULL,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (user_id,role_id)
);

COMMENT ON TABLE system_user_role IS '用户和角色关联表';
COMMENT ON COLUMN system_user_role.user_id IS '用户ID';
COMMENT ON COLUMN system_user_role.role_id IS '角色ID';
COMMENT ON COLUMN system_user_role.creator IS '创建者';
COMMENT ON COLUMN system_user_role.create_time IS '创建时间';
COMMENT ON COLUMN system_user_role.updater IS '更新者';
COMMENT ON COLUMN system_user_role.update_time IS '更新时间';
COMMENT ON COLUMN system_user_role.deleted IS '是否删除';

DROP TABLE IF EXISTS system_user;
CREATE TABLE system_user(
    id BIGINT NOT NULL,
    username VARCHAR(30) NOT NULL,
    password VARCHAR(200) NOT NULL,
    nickname VARCHAR(30) NOT NULL,
    remark VARCHAR(200),
    email VARCHAR(100),
    mobile VARCHAR(30),
    sex INTEGER DEFAULT  0,
    avatar VARCHAR(900),
    status INTEGER NOT NULL DEFAULT  0,
    login_ip VARCHAR(30),
    login_date TIMESTAMPTZ,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    tenant_id INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (id)
);

COMMENT ON TABLE system_user IS '用户表';
COMMENT ON COLUMN system_user.id IS '用户ID';
COMMENT ON COLUMN system_user.username IS '用户账号';
COMMENT ON COLUMN system_user.password IS '密码';
COMMENT ON COLUMN system_user.nickname IS '用户昵称';
COMMENT ON COLUMN system_user.remark IS '备注';
COMMENT ON COLUMN system_user.email IS '用户邮箱';
COMMENT ON COLUMN system_user.mobile IS '手机号码';
COMMENT ON COLUMN system_user.sex IS '用户性别（0男 1女 2未知 ）';
COMMENT ON COLUMN system_user.avatar IS '头像地址';
COMMENT ON COLUMN system_user.status IS '帐号状态（0正常 1停用）';
COMMENT ON COLUMN system_user.login_ip IS '最后登录IP';
COMMENT ON COLUMN system_user.login_date IS '最后登录时间';
COMMENT ON COLUMN system_user.creator IS '创建者';
COMMENT ON COLUMN system_user.create_time IS '创建时间';
COMMENT ON COLUMN system_user.updater IS '更新者';
COMMENT ON COLUMN system_user.update_time IS '更新时间';
COMMENT ON COLUMN system_user.deleted IS '是否删除（0正常 1删除）';
COMMENT ON COLUMN system_user.tenant_id IS '租户编号（默认 0）';

DROP TABLE IF EXISTS system_user_dept;
CREATE TABLE system_user_dept(
    user_id BIGINT NOT NULL,
    dept_id INTEGER NOT NULL,
    creator VARCHAR(100),
    create_time TIMESTAMPTZ NOT NULL,
    updater VARCHAR(100),
    update_time TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT  0,
    PRIMARY KEY (user_id,dept_id)
);

COMMENT ON TABLE system_user_dept IS '用户部门表';
COMMENT ON COLUMN system_user_dept.user_id IS '用户ID';
COMMENT ON COLUMN system_user_dept.dept_id IS '部门ID';
COMMENT ON COLUMN system_user_dept.creator IS '创建者';
COMMENT ON COLUMN system_user_dept.create_time IS '创建时间';
COMMENT ON COLUMN system_user_dept.updater IS '更新者';
COMMENT ON COLUMN system_user_dept.update_time IS '更新时间';
COMMENT ON COLUMN system_user_dept.deleted IS '是否删除（0正常 1删除）';

