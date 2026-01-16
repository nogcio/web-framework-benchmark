# gRPC Orders Aggregate Test Case (Realistic Inter-Service Scenario)

This test exercises gRPC calls with production-relevant aspects: binary protocol, deadlines, metadata, compression (optional), and in-memory aggregation without a database dependency. The data shape and aggregation logic mirror the JSON Analytics test case (`json_aggregate_spec.md`) to make results comparable across HTTP/1.1 JSON and gRPC.

**Quick summary:** same workload as JSON Analytics, but over gRPC to measure protocol overhead and metadata handling.

## Requirements

### Service and Method
- **Service**: `AnalyticsService` (no package defined)
- **RPC**: `AggregateOrders(AnalyticsRequest) returns (AggregateResult)`
- **Type**: Unary Call (Single request with a list of orders, single response)
- **Transport**: gRPC over HTTP/2 (POST).
- **Compression**: No compression required.
- **Deadline**: The client sets a `10s` deadline (`grpc-timeout: 10S`); the server must finish within it.
- **Metadata**: The client sends `x-client-id` (UUID). The server **must** echo it back in the response body.

### Proto Contract
```proto
syntax = "proto3";

message OrderItem {
  int32 quantity = 1;
  string category = 2;
  int64 price_cents = 3;
}

enum OrderStatus {
  UNKNOWN = 0;
  COMPLETED = 1;
  PENDING = 2;
  FAILED = 3;
}

message Order {
  string id = 1;
  OrderStatus status = 2;
  string country = 3;    // e.g. "US", "DE", "FR", "JP"
  repeated OrderItem items = 4;
}

message AnalyticsRequest {
  repeated Order orders = 1;
}

message AggregateResult {
  int32 processed_orders = 1;
  map<string, int64> amount_by_country = 2;   // cents
  map<string, int32> quantity_by_category = 3; // quantities
  string echoed_client_id = 4;                 // from metadata
}

service AnalyticsService {
  rpc AggregateOrders (AnalyticsRequest) returns (AggregateResult);
}
```

### Request Load Profile (matches JSON Analytics)
- Message count: same order of magnitude as the JSON Analytics test.
- Countries: `US, DE, FR, JP` (same distribution as JSON Analytics).
- Categories: `Electronics, Books, Clothing, Home`.
- Status: ~70% `completed`, the rest are ignored in aggregation (same ratio as JSON Analytics).
- Items: the runner generates `price_cents` and `quantity` per order item and validates the aggregates per request.
- Synthetic data only; no external dependencies (no DB required).

### Processing Logic (mirrors JSON Analytics)
1. Receive the `AnalyticsRequest` containing a list of `Order`s.
2. Iterate through the orders and consider only those with `status == "completed"` (same filter as JSON Analytics).
3. `processed_orders`: number of completed orders.
4. `amount_by_country[country]`: for each completed order, compute its `order_amount = sum(price_cents * quantity)` across its items; add this order_amount to the corresponding country bucket.
5. `quantity_by_category[category]`: sum `quantity` per category across items of completed orders.
6. Read metadata `x-client-id` and return it in `echoed_client_id`.
7. Return the `AggregateResult`.

### Response
- gRPC status: `OK` (code 0).
- Body: `AggregateResult` matching the runner’s expected aggregates exactly.
- Compression: None required.

## Verification (runner)
1. Sets metadata `x-client-id=<uuid>`, deadline 10s.
2. Sends a single `AnalyticsRequest` containing 100 generated `Order` messages.
3. Receives the single response and checks:
   - gRPC status `OK`.
   - `echoed_client_id` matches the sent value.
   - `processed_orders` and both maps match the expected control values exactly.

## Implementation Notes
- Metadata keys are case-sensitive in gRPC: use exactly `x-client-id`.
- Should work with TLS or plaintext; the test uses plaintext (insecure) in the local benchmark network by default.
