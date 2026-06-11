---
name: rust-ddd-test-generator
description: "Generate comprehensive test suites for Rust DDD (Domain-Driven Design) projects. This skill should be used when the user needs to create unit tests, integration tests, domain layer tests, application layer tests, repository tests, or event-driven tests for a Rust project following DDD architecture patterns. Covers all public-facing APIs, domain aggregates, value objects, domain services, application services, repositories, and event handlers to ensure the project is production-ready. Supports mockall, proptest, wiremock, insta, rstest, sqlx test macros, and cargo-tarpaulin for coverage reporting."
agent_created: true
---

# Rust DDD Test Generator

## Purpose

Generate production-grade, comprehensive test suites for Rust projects following Domain-Driven Design (DDD) architecture. The tests ensure correctness, completeness, and efficiency across all layers: Domain, Application, Infrastructure, and Interface.

## When to Use

- User asks to generate tests for a Rust DDD project
- User wants to ensure a Rust service/module is production-ready
- User requests unit tests, integration tests, or end-to-end tests for Rust code
- User wants to validate DDD aggregate invariants, domain events, or application commands
- User says things like "生成测试用例", "写测试", "确保功能正确", "生成测试基准"

## Workflow

### Step 1: Analyze the Project Structure

Before generating any tests, explore the project to understand its DDD layers:

```bash
# Explore project layout
find . -name "*.rs" | head -60
cat Cargo.toml
# Look for domain, application, infrastructure, interfaces directories
ls src/
```

Identify:
- **Domain layer**: Aggregates, Entities, Value Objects, Domain Services, Domain Events, Repositories (traits)
- **Application layer**: Commands, Queries, Application Services / Use Cases, DTOs
- **Infrastructure layer**: Repository implementations, external adapters, database models
- **Interface layer**: HTTP handlers, gRPC handlers, CLI

Load `references/ddd_test_patterns.md` for detailed patterns on testing each layer.

### Step 2: Discover All Public APIs

For each module, enumerate public-facing items that MUST be tested:

```bash
# Find all pub fn, pub struct, pub trait, pub enum
grep -rn "^pub " src/ --include="*.rs" | grep -v "mod\|use\|type\|const"
# Find all impl blocks
grep -rn "^impl " src/ --include="*.rs"
# Find domain events
grep -rn "DomainEvent\|Event\b" src/ --include="*.rs"
# Find aggregate roots
grep -rn "AggregateRoot\|#\[aggregate\]" src/ --include="*.rs"
```

### Step 3: Generate Tests per Layer

Follow the test generation rules in `references/ddd_test_patterns.md`.

**Coverage requirements:**
- Every public `fn` must have at least one happy-path test
- Every public `fn` that can fail must have at least one error-path test
- Every domain invariant must have a test proving it is enforced
- Every domain event must have a test asserting it is raised on the correct condition
- Every application service command/query must have both success and failure tests
- Every repository trait method must be tested via both mock (unit) and real impl (integration)

### Step 4: Write Test Files

**File placement conventions:**

| Layer | Test type | File location |
|-------|-----------|---------------|
| Domain entities/VOs | Unit | `src/domain/tests/` or inline `#[cfg(test)]` module |
| Domain services | Unit | `src/domain/tests/` |
| Application services | Unit (mocked repos) | `src/application/tests/` |
| Repository implementations | Integration | `tests/integration/` |
| HTTP/gRPC handlers | Integration | `tests/integration/` |
| Full workflow | E2E | `tests/e2e/` |

**Cargo.toml additions** (add if missing):

```toml
[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
mockall = "0.13"
fake = { version = "2", features = ["derive"] }
rstest = "0.23"
pretty_assertions = "1"
assert_matches = "1"
insta = "1"              # snapshot testing
proptest = "1"           # property-based testing (optional)
wiremock = "0.6"         # HTTP mock server (if project calls external HTTP)
sqlx = { version = "0.8", features = ["test"] }  # if using sqlx
```

### Step 5: Test Structure Standards

Each test file must follow this structure:

```rust
// ============================================================
// UNIT TESTS: <Module Name>
// Coverage: <list of items covered>
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // ---- Helpers / Fixtures --------------------------------
    fn make_valid_<aggregate>() -> <Aggregate> { ... }

    // ---- Happy Path ----------------------------------------
    #[test]  // or #[tokio::test] for async
    fn test_<method>_success() { ... }

    // ---- Error / Edge Cases --------------------------------
    #[test]
    fn test_<method>_returns_error_when_<condition>() { ... }

    // ---- Invariants ----------------------------------------
    #[test]
    fn test_<invariant_name>_is_enforced() { ... }

    // ---- Domain Events -------------------------------------
    #[test]
    fn test_<event>_is_raised_when_<condition>() { ... }
}
```

### Step 6: Mock Repository Pattern

For application layer unit tests, mock all repository and service traits:

```rust
use mockall::mock;
use mockall::predicate::*;

mock! {
    pub UserRepository {}
    impl UserRepository for UserRepository {
        async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, RepositoryError>;
        async fn save(&self, user: &User) -> Result<(), RepositoryError>;
    }
}

#[tokio::test]
async fn test_register_user_command_success() {
    let mut mock_repo = MockUserRepository::new();
    mock_repo
        .expect_find_by_id()
        .returning(|_| Ok(None));
    mock_repo
        .expect_save()
        .times(1)
        .returning(|_| Ok(()));

    let svc = UserApplicationService::new(Arc::new(mock_repo));
    let cmd = RegisterUserCommand { email: "test@example.com".to_string(), ... };
    let result = svc.register(cmd).await;
    assert!(result.is_ok());
}
```

### Step 7: Integration Test Patterns

For integration tests that hit real databases or external services:

```rust
// tests/integration/user_repository_test.rs
use sqlx::PgPool;

#[sqlx::test(fixtures("users"))]
async fn test_find_user_by_email(pool: PgPool) {
    let repo = SqlxUserRepository::new(pool);
    let result = repo.find_by_email("existing@example.com").await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().email(), "existing@example.com");
}
```

Use `wiremock` for tests calling external HTTP APIs:

```rust
#[tokio::test]
async fn test_payment_gateway_charge_success() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/charges"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&charge_response()))
        .mount(&mock_server)
        .await;

    let gateway = HttpPaymentGateway::new(&mock_server.uri());
    let result = gateway.charge(Money::new(100, Currency::CNY)).await;
    assert!(result.is_ok());
}
```

### Step 8: Property-Based Tests for Value Objects

For Value Objects with complex validation rules, add property-based tests:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_email_vo_rejects_invalid(s in "[^@]{1,100}") {
        // strings without @ should fail
        prop_assert!(Email::new(s).is_err());
    }

    #[test]
    fn test_money_amount_never_negative(amount in 0u64..1_000_000) {
        let m = Money::new(amount, Currency::CNY);
        prop_assert!(m.is_ok());
        prop_assert!(m.unwrap().amount() >= 0);
    }
}
```

### Step 9: Test Coverage Check

After generating tests, run coverage to ensure completeness:

```bash
# Install tarpaulin (Rust coverage tool)
cargo install cargo-tarpaulin

# Run all tests with coverage
cargo tarpaulin --out Html --output-dir coverage/ --exclude-files "src/main.rs"

# Run specific test types
cargo test --lib                    # unit tests only
cargo test --test '*'               # integration tests only
cargo test                          # all tests
cargo test -- --nocapture           # with stdout output
```

**Minimum coverage targets:**
- Domain layer: ≥ 90%
- Application layer: ≥ 85%
- Infrastructure layer: ≥ 75%
- Overall: ≥ 80%

### Step 10: CI-Ready Test Configuration

Add `.cargo/config.toml` for test configuration:

```toml
[alias]
test-unit = "test --lib --bins"
test-integration = "test --tests"
test-all = "test --all-features"
test-coverage = "tarpaulin --out Xml --output-dir coverage/"
```

Add `Makefile` targets or provide commands to run the full test suite in CI.

## Key DDD Testing Principles

1. **Aggregate invariants are sacred** — every business rule in an aggregate must have a failing test that proves the invariant is enforced.
2. **Test the domain, not the framework** — domain layer tests must not depend on databases, HTTP, or any infrastructure.
3. **Events are contracts** — if a domain event is part of the public API, test its payload schema.
4. **Ports, not adapters** — application service tests depend on port interfaces (traits), never on concrete adapters.
5. **Fixture factories over raw constructors** — use builder pattern or factory functions for test data to keep tests readable.
6. **One assertion per concept** — multiple asserts per test are fine if they all verify the same logical concept.
7. **Test names are documentation** — `test_order_cannot_be_shipped_when_payment_pending` is far better than `test_ship_order`.

## Output Format

After generating tests, always provide:

1. A summary table of test coverage by layer
2. The list of generated test files and their locations  
3. Commands to run the tests
4. Any missing dev-dependencies that need to be added to `Cargo.toml`
5. Suggestions for additional property-based or fuzz tests if applicable
