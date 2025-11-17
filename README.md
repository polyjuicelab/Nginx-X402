# nginx-x402

Nginx FFI module for x402 HTTP micropayment protocol.

## Overview

This project provides C-compatible FFI bindings that allow Nginx to verify x402 payments and return 402 Payment Required responses. The module is designed to be used with Nginx C modules.

## Architecture

The Nginx integration uses a hybrid approach:
- **FFI Module** (`src/ffi.rs`) - C-compatible functions for payment operations
- **Nginx C Module** - C module that calls FFI functions (to be implemented)

## Prerequisites

- Rust toolchain (latest stable)
- Nginx development headers (for C module integration)
- `rust-x402` library (dependency)

## Building

```bash
# Build the library
cargo build --release
```

The library will be built as:
- `libnginx_x402.so` (Linux)
- `libnginx_x402.dylib` (macOS)

## Usage

### From Nginx C Module

```c
#include "nginx/x402_ffi.h"

// Verify payment
char result[4096];
size_t result_len = sizeof(result);
int status = x402_verify_payment(
    payment_b64, requirements_json, facilitator_url,
    result, &result_len
);

if (status == 0) {
    // Payment is valid, proceed with request
} else if (status == 2) {
    // Payment verification failed, return 402
}
```

## FFI Functions

See `nginx/x402_ffi.h` for complete function documentation.

### Main Functions

- `x402_verify_payment` - Verify a payment payload
- `x402_create_requirements` - Create payment requirements JSON
- `x402_generate_paywall_html` - Generate HTML paywall page
- `x402_generate_json_response` - Generate JSON 402 response
- `x402_is_browser_request` - Detect browser vs API client
- `x402_free_string` - Free allocated strings

## Configuration

See `nginx/example.conf` for Nginx configuration examples.

## Testing

Run all tests:

```bash
cargo test
```

## License

AGPL-3.0

