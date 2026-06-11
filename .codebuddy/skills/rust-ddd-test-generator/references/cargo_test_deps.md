# Cargo.toml Dev-Dependency Template for Rust DDD Projects
# ──────────────────────────────────────────────────────────
# Copy the relevant sections into your project's Cargo.toml.

[dev-dependencies]
# Async runtime for async tests
tokio = { version = "1", features = ["full", "test-util"] }

# Mock framework for trait mocking (application layer unit tests)
mockall = "0.13"

# Fake data generation (test fixtures)
fake = { version = "2", features = ["derive"] }

# Parameterized tests
rstest = "0.23"

# Prettier assertion output
pretty_assertions = "1"

# Pattern matching assertions
assert_matches = "1"

# Snapshot testing (for DTO/serialization tests)
insta = { version = "1", features = ["json", "yaml"] }

# Property-based testing
proptest = "1"

# HTTP mock server (for tests calling external HTTP services)
wiremock = "0.6"

# If using SQLx: database test support
# sqlx = { version = "0.8", features = ["test", "postgres", "runtime-tokio-native-tls"] }

# If using testcontainers for real DB integration tests
# testcontainers = "0.22"
# testcontainers-modules = { version = "0.11", features = ["postgres", "redis"] }


# ──────────────────────────────────────────────────────────
# .cargo/config.toml aliases for running test suites
# ──────────────────────────────────────────────────────────
# [alias]
# test-unit        = "test --lib --bins"
# test-integration = "test --tests"
# test-all         = "test --all-features"
# test-coverage    = "tarpaulin --out Html --output-dir coverage/ --exclude-files src/main.rs"


# ──────────────────────────────────────────────────────────
# insta snapshot review
# ──────────────────────────────────────────────────────────
# After running tests with insta, review/accept snapshots:
#   cargo insta review


# ──────────────────────────────────────────────────────────
# proptest configuration (proptest.toml or Cargo.toml)
# ──────────────────────────────────────────────────────────
# [profile.test]
# opt-level = 1   # faster proptest execution


# ──────────────────────────────────────────────────────────
# Coverage with tarpaulin
# ──────────────────────────────────────────────────────────
# Install: cargo install cargo-tarpaulin
# Run:     cargo tarpaulin --out Html --output-dir coverage/
# Run CI:  cargo tarpaulin --out Xml --fail-under 80
