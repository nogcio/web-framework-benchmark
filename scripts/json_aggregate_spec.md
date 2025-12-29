# JSON Analytics Test Case

This test case verifies JSON request parsing, in-memory aggregation, and JSON response serialization.

## Requirements

### Endpoint
- **URL**: `/json/aggregate`
- **Method**: `POST`
- **Request Headers**: `Content-Type: application/json`

### Request Body
The request body must be a JSON array of orders.

#### Order
- `status` (string)
- `amount` (integer, cents)
- `country` (string)
- `items` (array of `OrderItem`)

#### OrderItem
- `quantity` (integer)
- `category` (string)

Notes:
- All monetary values are integer cents.
- The runner generates `amount` as the sum of `price * quantity` for each item, but implementations should treat the request body as the source of truth.

### Processing Logic
The service must compute analytics using only orders where `status == "completed"`.

1. **processedOrders**: the number of orders with `status == "completed"`.
2. **results**: a per-country sum of `amount` across completed orders.
   - Key: `country` string
   - Value: integer cents sum
3. **categoryStats**: a per-category sum of `quantity` across items belonging to completed orders.
   - Key: `category` string
   - Value: integer quantity sum

### Response
- **Status Code**: `200 OK`
- **Headers**: `Content-Type: application/json`
- **Body**: JSON object with exactly the following fields:
  - `processedOrders` (integer)
  - `results` (object map: string -> integer)
  - `categoryStats` (object map: string -> integer)

Example shape:

```json
{
  "processedOrders": 167,
  "results": {
    "US": 123456,
    "DE": 7890
  },
  "categoryStats": {
    "Electronics": 250,
    "Books": 180
  }
}
```

## Verification Logic
The test runner performs the following checks:
1. Sends a `POST` request to `/json/aggregate` with the generated array of orders as JSON.
2. Asserts that the HTTP status code is `200`.
3. Asserts that the `Content-Type` response header contains `application/json`.
4. Parses the response JSON as:
   - `processedOrders: usize`
   - `results: map<string, int64>`
   - `categoryStats: map<string, int32>`
5. Asserts:
   - `processedOrders` equals the expected number of completed orders.
   - For each country in `["US", "DE", "FR", "UK", "JP"]`, `results[country]` equals the expected sum.
   - For each category in `["Electronics", "Books", "Clothing", "Home"]`, `categoryStats[category]` equals the expected count.