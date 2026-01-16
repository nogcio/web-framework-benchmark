# Plain Text Test Case

This test case verifies the implementation of the simplest possible HTTP response.

**Why this matters:** it establishes a baseline for raw HTTP handling before adding JSON, DB, or file I/O overhead.

## Requirements

### Endpoint
- **URL**: `/plaintext`
- **Method**: `GET`

### Response
- **Status Code**: `200 OK`
- **Headers**: `Content-Type: text/plain`
- **Body**: Must be exactly `Hello, World!`

### Verification Logic
The test runner performs the following checks:
1. Sends a `GET` request to `/plaintext`.
2. Asserts that the HTTP status code is `200`.
3. Asserts that the `Content-Type` header is `text/plain`.
4. Asserts that the response body is exactly the string `Hello, World!`.
