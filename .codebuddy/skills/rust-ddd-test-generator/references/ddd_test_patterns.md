# DDD Test Patterns for Rust

## Overview

This reference covers concrete test patterns for each DDD layer in a Rust project.
Load this file when generating tests to ensure all patterns are applied correctly.

---

## 1. Domain Layer Tests

### 1.1 Value Object Tests

Value Objects must test:
- Valid construction (all valid inputs produce correct VO)
- Invalid construction (all invalid inputs are rejected with correct error)
- Equality semantics (`PartialEq` / `Eq`)
- Ordering (if `PartialOrd` / `Ord` is implemented)
- Display / serialization (if applicable)

```rust
#[cfg(test)]
mod email_tests {
    use super::*;

    // ---- Valid Construction ----
    #[test]
    fn test_email_accepts_valid_format() {
        let email = Email::new("user@example.com").unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_email_normalizes_to_lowercase() {
        let email = Email::new("USER@EXAMPLE.COM").unwrap();
        assert_eq!(email.as_str(), "user@example.com");
    }

    // ---- Invalid Construction ----
    #[test]
    fn test_email_rejects_missing_at_sign() {
        assert!(Email::new("userexample.com").is_err());
    }

    #[test]
    fn test_email_rejects_empty_string() {
        assert!(Email::new("").is_err());
    }

    #[test]
    fn test_email_rejects_multiple_at_signs() {
        assert!(Email::new("a@@b.com").is_err());
    }

    // ---- Equality ----
    #[test]
    fn test_two_emails_with_same_value_are_equal() {
        let a = Email::new("x@y.com").unwrap();
        let b = Email::new("x@y.com").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_two_emails_with_different_values_are_not_equal() {
        let a = Email::new("x@y.com").unwrap();
        let b = Email::new("z@y.com").unwrap();
        assert_ne!(a, b);
    }
}
```

### 1.2 Entity Tests

Entities must test:
- Identity (two entities with same ID are equal regardless of other fields)
- State transitions (valid transitions succeed, invalid ones return errors)
- Business rules / invariants are enforced
- Domain events are raised when expected

```rust
#[cfg(test)]
mod order_entity_tests {
    use super::*;
    use crate::domain::order::{Order, OrderStatus, OrderId};

    fn make_pending_order() -> Order {
        Order::new(
            OrderId::new(),
            CustomerId::from("cust-1"),
            vec![OrderItem::new(ProductId::from("prod-1"), 2, Money::cny(100))],
        ).unwrap()
    }

    // ---- Identity ----
    #[test]
    fn test_orders_with_same_id_are_equal() {
        let id = OrderId::new();
        let a = Order::reconstitute(id.clone(), /* ... */);
        let b = Order::reconstitute(id.clone(), /* ... */);
        assert_eq!(a.id(), b.id());
    }

    // ---- State Transitions ----
    #[test]
    fn test_confirm_order_transitions_to_confirmed() {
        let mut order = make_pending_order();
        order.confirm().unwrap();
        assert_eq!(order.status(), OrderStatus::Confirmed);
    }

    #[test]
    fn test_cannot_confirm_already_confirmed_order() {
        let mut order = make_pending_order();
        order.confirm().unwrap();
        let result = order.confirm();
        assert!(result.is_err());
        // Be specific about the error type
        assert!(matches!(result, Err(OrderError::InvalidStateTransition { .. })));
    }

    #[test]
    fn test_cannot_ship_unconfirmed_order() {
        let mut order = make_pending_order();
        let result = order.ship();
        assert!(result.is_err());
    }

    // ---- Invariants ----
    #[test]
    fn test_order_must_have_at_least_one_item() {
        let result = Order::new(OrderId::new(), CustomerId::from("c1"), vec![]);
        assert!(result.is_err());
    }

    // ---- Domain Events ----
    #[test]
    fn test_confirming_order_raises_order_confirmed_event() {
        let mut order = make_pending_order();
        order.confirm().unwrap();
        let events = order.take_domain_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], DomainEvent::OrderConfirmed(e) if e.order_id == order.id()));
    }
}
```

### 1.3 Aggregate Root Tests

In addition to entity tests, aggregate roots must test:
- Consistency boundary enforcement (child entities cannot violate aggregate invariants)
- Event sourcing / event collection (if applicable)

```rust
#[cfg(test)]
mod shopping_cart_aggregate_tests {
    use super::*;

    // ---- Consistency Boundary ----
    #[test]
    fn test_cart_total_equals_sum_of_all_items() {
        let mut cart = ShoppingCart::new(CartId::new(), CustomerId::from("c1"));
        cart.add_item(ProductId::from("p1"), 2, Money::cny(50)).unwrap();
        cart.add_item(ProductId::from("p2"), 1, Money::cny(100)).unwrap();
        assert_eq!(cart.total(), Money::cny(200));
    }

    #[test]
    fn test_cannot_add_same_product_twice_without_merge() {
        let mut cart = ShoppingCart::new(CartId::new(), CustomerId::from("c1"));
        cart.add_item(ProductId::from("p1"), 1, Money::cny(50)).unwrap();
        cart.add_item(ProductId::from("p1"), 2, Money::cny(50)).unwrap();
        // Should merge into quantity 3, not two separate items
        assert_eq!(cart.items().len(), 1);
        assert_eq!(cart.items()[0].quantity(), 3);
    }
}
```

### 1.4 Domain Service Tests

Domain services that coordinate multiple aggregates:

```rust
#[cfg(test)]
mod pricing_service_tests {
    use super::*;

    #[test]
    fn test_apply_discount_returns_correct_discounted_price() {
        let service = PricingService::new();
        let price = Money::cny(1000);
        let discount = Discount::percentage(10);
        let result = service.apply_discount(price, discount);
        assert_eq!(result, Money::cny(900));
    }

    #[test]
    fn test_discount_cannot_exceed_100_percent() {
        let service = PricingService::new();
        let result = Discount::percentage(101);
        assert!(result.is_err());
    }
}
```

---

## 2. Application Layer Tests

### 2.1 Command Handler Tests (mocked repos)

```rust
use mockall::predicate::*;

#[tokio::test]
async fn test_create_order_command_success() {
    let mut mock_order_repo = MockOrderRepository::new();
    let mut mock_inventory_repo = MockInventoryRepository::new();
    let mut mock_event_bus = MockDomainEventBus::new();

    mock_inventory_repo
        .expect_check_availability()
        .with(eq(ProductId::from("p1")), eq(2u32))
        .returning(|_, _| Ok(true));

    mock_order_repo
        .expect_save()
        .once()
        .returning(|_| Ok(()));

    mock_event_bus
        .expect_publish()
        .once()
        .returning(|_| Ok(()));

    let svc = OrderApplicationService::new(
        Arc::new(mock_order_repo),
        Arc::new(mock_inventory_repo),
        Arc::new(mock_event_bus),
    );

    let cmd = CreateOrderCommand {
        customer_id: "cust-1".to_string(),
        items: vec![OrderItemDto { product_id: "p1".to_string(), quantity: 2, unit_price: 100 }],
    };

    let result = svc.create_order(cmd).await;
    assert!(result.is_ok());
    let order_id = result.unwrap();
    assert!(!order_id.is_empty());
}

#[tokio::test]
async fn test_create_order_fails_when_product_out_of_stock() {
    let mut mock_inventory_repo = MockInventoryRepository::new();
    mock_inventory_repo
        .expect_check_availability()
        .returning(|_, _| Ok(false));

    let mock_order_repo = MockOrderRepository::new();
    let mock_event_bus = MockDomainEventBus::new();

    let svc = OrderApplicationService::new(
        Arc::new(mock_order_repo),
        Arc::new(mock_inventory_repo),
        Arc::new(mock_event_bus),
    );

    let cmd = CreateOrderCommand {
        customer_id: "cust-1".to_string(),
        items: vec![OrderItemDto { product_id: "p1".to_string(), quantity: 999, unit_price: 100 }],
    };

    let result = svc.create_order(cmd).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ApplicationError::InsufficientInventory { .. })));
}
```

### 2.2 Query Handler Tests

```rust
#[tokio::test]
async fn test_get_order_returns_dto_when_found() {
    let order_id = OrderId::new();
    let mut mock_repo = MockOrderReadRepository::new();

    mock_repo
        .expect_find_by_id()
        .with(eq(order_id.clone()))
        .returning(|id| Ok(Some(OrderReadModel {
            id: id.to_string(),
            status: "Pending".to_string(),
            total: 200,
            ..Default::default()
        })));

    let query_svc = OrderQueryService::new(Arc::new(mock_repo));
    let result = query_svc.get_order(order_id).await;

    assert!(result.is_ok());
    let dto = result.unwrap();
    assert_eq!(dto.status, "Pending");
    assert_eq!(dto.total, 200);
}

#[tokio::test]
async fn test_get_order_returns_not_found_error_when_missing() {
    let mut mock_repo = MockOrderReadRepository::new();
    mock_repo.expect_find_by_id().returning(|_| Ok(None));

    let query_svc = OrderQueryService::new(Arc::new(mock_repo));
    let result = query_svc.get_order(OrderId::new()).await;

    assert!(matches!(result, Err(ApplicationError::NotFound { .. })));
}
```

---

## 3. Infrastructure Layer Tests

### 3.1 Repository Implementation Tests (Integration)

```rust
// tests/integration/order_repository_test.rs
use sqlx::PgPool;
use your_app::infrastructure::repositories::SqlxOrderRepository;
use your_app::domain::order::*;

#[sqlx::test]
async fn test_save_and_find_order(pool: PgPool) {
    let repo = SqlxOrderRepository::new(pool);
    let order = Order::new(
        OrderId::new(),
        CustomerId::from("cust-1"),
        vec![OrderItem::new(ProductId::from("p1"), 1, Money::cny(100))],
    ).unwrap();

    repo.save(&order).await.unwrap();

    let found = repo.find_by_id(order.id()).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id(), order.id());
    assert_eq!(found.status(), order.status());
}

#[sqlx::test]
async fn test_find_by_id_returns_none_for_nonexistent_order(pool: PgPool) {
    let repo = SqlxOrderRepository::new(pool);
    let result = repo.find_by_id(&OrderId::new()).await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test]
async fn test_update_order_status_is_persisted(pool: PgPool) {
    let repo = SqlxOrderRepository::new(pool);
    let mut order = Order::new(/* ... */).unwrap();
    repo.save(&order).await.unwrap();

    order.confirm().unwrap();
    repo.save(&order).await.unwrap();

    let found = repo.find_by_id(order.id()).await.unwrap().unwrap();
    assert_eq!(found.status(), OrderStatus::Confirmed);
}
```

### 3.2 External HTTP Adapter Tests

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

#[tokio::test]
async fn test_payment_adapter_charge_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/charges"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "id": "ch_123",
                    "status": "succeeded",
                    "amount": 10000
                }))
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let adapter = StripePaymentAdapter::new(mock_server.uri(), "test-key");
    let result = adapter.charge(Money::cny(100)).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().charge_id(), "ch_123");
}

#[tokio::test]
async fn test_payment_adapter_handles_gateway_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(402).set_body_json(serde_json::json!({
            "error": { "code": "card_declined", "message": "Your card was declined." }
        })))
        .mount(&mock_server)
        .await;

    let adapter = StripePaymentAdapter::new(mock_server.uri(), "test-key");
    let result = adapter.charge(Money::cny(100)).await;

    assert!(matches!(result, Err(PaymentError::CardDeclined)));
}
```

---

## 4. Interface Layer Tests

### 4.1 HTTP Handler Tests (Axum / Actix-web)

```rust
// tests/integration/http_order_handler_test.rs
use axum::http::{Request, StatusCode};
use axum::body::Body;
use tower::ServiceExt;
use serde_json::json;

async fn create_test_app() -> Router {
    // Use in-memory DB or test containers
    let pool = test_db_pool().await;
    build_router(pool)
}

#[tokio::test]
async fn test_post_orders_returns_201() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/orders")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-token")
        .body(Body::from(json!({
            "customerId": "cust-1",
            "items": [{ "productId": "prod-1", "quantity": 2 }]
        }).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body()).await.unwrap();
    let order: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(order["orderId"].is_string());
}

#[tokio::test]
async fn test_get_order_returns_404_when_not_found() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/orders/non-existent-id")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_post_orders_returns_400_for_empty_items() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/orders")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "customerId": "cust-1",
            "items": []
        }).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

---

## 5. Domain Event Tests

### 5.1 Event Handler Tests

```rust
#[tokio::test]
async fn test_order_confirmed_event_triggers_inventory_reservation() {
    let mut mock_inventory_svc = MockInventoryService::new();
    mock_inventory_svc
        .expect_reserve()
        .with(eq(ProductId::from("p1")), eq(2u32))
        .once()
        .returning(|_, _| Ok(ReservationId::new()));

    let handler = OrderConfirmedEventHandler::new(Arc::new(mock_inventory_svc));

    let event = OrderConfirmedEvent {
        order_id: OrderId::new(),
        items: vec![OrderItemSnapshot { product_id: ProductId::from("p1"), quantity: 2 }],
        confirmed_at: Utc::now(),
    };

    let result = handler.handle(event).await;
    assert!(result.is_ok());
}
```

---

## 6. Test Data Builders

Use builder pattern for complex test data:

```rust
pub struct OrderBuilder {
    id: OrderId,
    customer_id: CustomerId,
    items: Vec<OrderItem>,
    status: OrderStatus,
}

impl OrderBuilder {
    pub fn new() -> Self {
        Self {
            id: OrderId::new(),
            customer_id: CustomerId::from("test-customer"),
            items: vec![
                OrderItem::new(ProductId::from("prod-1"), 1, Money::cny(100)).unwrap()
            ],
            status: OrderStatus::Pending,
        }
    }

    pub fn with_id(mut self, id: OrderId) -> Self { self.id = id; self }
    pub fn with_customer(mut self, id: &str) -> Self { self.customer_id = CustomerId::from(id); self }
    pub fn with_items(mut self, items: Vec<OrderItem>) -> Self { self.items = items; self }
    pub fn confirmed(mut self) -> Self { self.status = OrderStatus::Confirmed; self }

    pub fn build(self) -> Order {
        Order::reconstitute(self.id, self.customer_id, self.items, self.status)
    }
}
```

---

## 7. Property-Based Tests

Use `proptest` for Value Objects and pure domain logic:

```rust
use proptest::prelude::*;

proptest! {
    // Test that Money addition is commutative
    #[test]
    fn test_money_addition_is_commutative(a in 0u64..100_000, b in 0u64..100_000) {
        let ma = Money::cny(a);
        let mb = Money::cny(b);
        prop_assert_eq!(ma.add(mb).unwrap(), mb.add(ma).unwrap());
    }

    // Test that valid UUIDs are always accepted as IDs
    #[test]
    fn test_valid_uuid_always_accepted_as_order_id(s in "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}") {
        prop_assert!(OrderId::from_str(&s).is_ok());
    }
}
```

---

## 8. Snapshot Tests

Use `insta` for complex DTO / serialization tests:

```rust
use insta::assert_json_snapshot;

#[test]
fn test_order_dto_serialization() {
    let order = OrderBuilder::new()
        .with_id(OrderId::from_str("00000000-0000-4000-8000-000000000001").unwrap())
        .build();

    let dto = OrderDto::from(order);
    assert_json_snapshot!(dto);
}
```

---

## 9. Test Organization Checklist

For each DDD aggregate / bounded context, ensure:

- [ ] All Value Objects: valid + invalid construction + equality
- [ ] All Entities: identity + all state transitions (happy + error) + invariants
- [ ] All Aggregates: consistency rules + domain events emitted
- [ ] All Domain Services: pure logic tests (no mocks needed)
- [ ] All Application Commands: success path + all error paths + mock expectations
- [ ] All Application Queries: found + not found + pagination if applicable
- [ ] All Repository Traits: at least one integration test for each method
- [ ] All External Adapters: success + error + timeout (if applicable)
- [ ] All HTTP/gRPC Handlers: 2xx + 4xx + 5xx for each endpoint
- [ ] All Domain Event Handlers: correct side effects triggered

---

## 10. Common Test Error Patterns to Cover

| Scenario | Expected Error |
|----------|---------------|
| Entity not found | `NotFound` / `ResourceNotFound` |
| Duplicate entity | `AlreadyExists` / `Conflict` |
| Invalid state transition | `InvalidStateTransition` |
| Invariant violated | `DomainRuleViolated` / `InvalidInput` |
| Unauthorized action | `Unauthorized` / `Forbidden` |
| External service failure | `ExternalServiceError` / `ServiceUnavailable` |
| Concurrency conflict | `OptimisticLockError` / `ConcurrencyConflict` |
| Validation failure | `ValidationError` with field details |
