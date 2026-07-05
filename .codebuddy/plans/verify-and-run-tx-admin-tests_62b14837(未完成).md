---
name: verify-and-run-tx-admin-tests
overview: 按 DDD 分层（domain→app→api）检查测试用例正确性和覆盖率，然后执行测试验证
todos:
  - id: check-domain-compile
    content: "[skill:rust-ddd-test-generator] 检查 admin_domain 测试编译：cargo check -p admin_domain --tests"
    status: pending
  - id: run-domain-tests
    content: 执行 admin_domain 全部单元测试：cargo test -p admin_domain 并记录结果
    status: pending
    dependencies:
      - check-domain-compile
  - id: review-domain-coverage
    content: 审查 domain 层测试覆盖完整性（对照 DDD 测试清单），输出覆盖评估与补全建议
    status: pending
    dependencies:
      - run-domain-tests
  - id: check-app-compile
    content: 检查 admin_app 集成测试编译：cargo check -p admin_app --tests
    status: pending
    dependencies:
      - review-domain-coverage
  - id: run-app-tests
    content: 执行 admin_app 全部集成测试：cargo test -p admin_app 并记录结果
    status: pending
    dependencies:
      - check-app-compile
  - id: review-app-coverage
    content: 审查 app 层测试覆盖完整性，输出覆盖评估与补全建议
    status: pending
    dependencies:
      - run-app-tests
  - id: check-api-auxiliary
    content: 检查 admin_api/admin_proto/admin_macros 测试编译，执行辅助 crate 测试
    status: pending
    dependencies:
      - review-app-coverage
  - id: summary-report
    content: 汇总全部测试结果，输出分层覆盖报告和补全建议
    status: pending
    dependencies:
      - check-api-auxiliary
---

## 需求分析

用户要求对 `examples/tx_admin` 项目按 DDD 分层（domain -> app -> api）依次检查测试用例，确保测试的正确性和覆盖完整性，再执行测试用例进行验证。项目已从旧 API（`#[tx_comp]`、`CompInit`、`async_method!`）迁移到新 API（`#[derive(Component)]`），需要确认迁移未破坏现有测试。

分层检查的内容：

- **Domain 层**：admin_domain crate，15 个测试模块约 216 个测试用例，均为纯单元测试（closure-based mock repo）
- **App 层**：admin_app crate，11 个集成测试文件，使用 SQLite in-memory + Toasty 的真实持久化
- **API 层**：admin_api crate，gRPC 集成测试（需要服务器运行）
- **辅助 crate**：admin_proto、admin_macros 的测试检查

## 技术方案

### 技术栈

- Rust + Cargo 工作区
- 测试框架：tokio::test（异步）、std test（同步）
- 数据库：Toasty ORM + SQLite in-memory
- Mock 模式：closure-based function mock（domain 层），真实 Toasty 实现（app 层）
- 辅助 skill：rust-ddd-test-generator

### 验证策略

按 DDD 分层自底向上验证，每层完成编译检查 + 测试执行 + 覆盖审查后进入下一层。

#### 1. Domain 层（admin_domain）

- **编译检查**：`cargo check -p admin_domain --tests` 验证所有测试编译通过
- **测试执行**：`cargo test -p admin_domain` 执行全部单元测试
- **覆盖审查**：对照 skill 参考文档中的 DDD 测试清单，逐模块审查覆盖完整性
- **关键验证点**：聚合根领域事件、值对象枚举、服务层成功/错误路径、password 安全模块

#### 2. App 层（admin_app）

- **编译检查**：`cargo check -p admin_app --tests`（需要 `admin_infra` dev-dependency + Toasty sqlite）
- **测试执行**：`cargo test -p admin_app` 执行全部集成测试
- **覆盖审查**：检查每个模块的 CRUD 操作、跨聚合校验、DTO 转换
- **关键验证点**：真实数据库的持久化正确性、种子数据完整性

#### 3. API 层（admin_api）& 辅助 crate

- **admin_proto** + **admin_macros**：`cargo check --tests` 检查和执行测试
- **admin_api**：gRPC 测试需要运行服务器，仅检查编译通过

#### 4. Coverage 差距分析

使用 [skill:rust-ddd-test-generator] 工具的测试清单标准，评估每层是否存在函数无对应测试用例的覆盖缺口，输出补全建议。

### 注意事项

- 测试文件不直接使用 `#[derive(Component)]`，而是通过 `Service::new(Arc::new(MockRepo))` 直接构造
- API 迁移不会破坏现有测试逻辑，但仍需编译验证
- `integration_test.rs`（0B）是空文件，需要确认是否需要移除或填充
- gRPC 测试需要先启动服务器，不适合自动化测试流程（仅编译检查）

## Agent Extensions

### Skill

- **rust-ddd-test-generator**
- Purpose: 使用该 skill 的 DDD 测试模式参考文档（`ddd_test_patterns.md`）中的完整测试清单，逐模块评估 domain/app/infra 层的覆盖完整性，识别覆盖率缺口
- Expected outcome: 输出各层覆盖完整性评估表和补全建议

### SubAgent

- **code-explorer**
- Purpose: 在需要跨多个目录搜索或读取大量文件时使用，例如搜索未覆盖的公共函数、统计测试数量等
- Expected outcome: 提高大规模代码浏览的效率