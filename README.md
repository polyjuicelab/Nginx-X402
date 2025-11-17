# nginx-x402

<div align="center">
  <img src="logo.png" alt="nginx-x402 logo" width="200">
</div>

Pure Rust implementation of Nginx module for x402 HTTP micropayment protocol.

## Overview

This project provides a **pure Rust** Nginx module that can be loaded directly by Nginx. The module implements the x402 payment verification protocol using the official [ngx-rust](https://github.com/nginx/ngx-rust) crate.

## Architecture

Uses the official [ngx-rust](https://github.com/nginx/ngx-rust) crate to implement the module entirely in Rust. **No C wrapper needed!**

- ✅ 100% Rust implementation
- ✅ Type-safe Nginx API bindings
- ✅ Official Nginx support
- ⚠️ Requires Nginx source code for building

## Prerequisites

- Rust toolchain (latest stable, 1.81.0+)
- `rust-x402` library (from crates.io)
- **Option A (Recommended)**: Nginx source code (same version as production)
  - Set `NGINX_SOURCE_DIR` or `NGINX_BUILD_DIR` environment variable
- **Option B (Convenient)**: Use `vendored` feature to auto-download Nginx source
  - No manual Nginx source needed, but may not match production version
- libclang (for bindgen)

## Building

### Method A: With Nginx Source (Recommended for Production)

```bash
# Set Nginx source directory (use same version as production)
export NGINX_SOURCE_DIR=/path/to/nginx-1.29.1

# Build (requires Nginx source or vendored feature)
cargo build --release

# The module will be at:
# target/aarch64-apple-darwin/release/libnginx_x402.dylib (macOS)
# target/x86_64-unknown-linux-gnu/release/libnginx_x402.so (Linux)
```

### Method B: Auto-download Nginx Source (Convenient for Development)

```bash
# No Nginx source needed! ngx-rust will download it automatically
cargo build --release --features vendored
```

**Note**: The `vendored` feature downloads a default Nginx version. For production, use Method A with the exact Nginx version you'll deploy.

### Load Module in Nginx

Add to your `nginx.conf`:

```nginx
load_module /path/to/libnginx_x402.so;

http {
    server {
        location /protected {
            x402 on;
            x402_amount 0.0001;
            x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
            x402_facilitator_url https://x402.org/facilitator;
        }
    }
}
```

## Configuration Directives

- `x402 on|off` - Enable/disable x402 payment verification
- `x402_amount <amount>` - Payment amount (e.g., "0.0001")
- `x402_pay_to <address>` - Recipient wallet address
- `x402_facilitator_url <url>` - Facilitator service URL
- `x402_description <text>` - Payment description
- `x402_network <network>` - Network identifier (e.g., "base", "base-sepolia")
- `x402_resource <path>` - Resource path (default: request URI)
- `x402_timeout <seconds>` - Timeout for facilitator service requests in seconds (1-300, default: 10). Note: This is for facilitator API calls only, not for Nginx HTTP request timeouts (use `proxy_read_timeout` etc. in nginx.conf for those).
- `x402_facilitator_fallback <mode>` - Fallback behavior when facilitator fails: `error` (return 500, default) or `pass` (pass through as if middleware doesn't exist)

## Testing

The project includes comprehensive Rust unit tests that cover all core functionality:

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test ngx_module_tests
cargo test --test config_validation_tests
cargo test --test browser_detection_tests
```

### Test Coverage

The test suite includes:
- **Core Logic Tests** (`ngx_module_tests.rs`) - Payment verification, requirements creation, 402 responses
- **Configuration Validation** (`config_validation_tests.rs`) - Config parsing and validation
- **Browser Detection** (`browser_detection_tests.rs`) - Request type detection
- **Security Tests** (`security_tests.rs`) - Payment header validation, security checks
- **Resource Path Validation** (`resource_path_validation_tests.rs`) - Path traversal prevention
- **Timeout Tests** (`timeout_tests.rs`) - Network timeout handling
- **Runtime Tests** (`runtime_tests.rs`) - Tokio runtime initialization
- **Client Pool Tests** (`facilitator_client_pool_tests.rs`) - Connection pooling
- **Validation Unit Tests** (`validation_unit_tests.rs`) - Input validation functions
- **Logging Tests** (`logging_tests.rs`) - Logging functionality

All tests can run without requiring Nginx source code or a running Nginx instance.

## How It Works

### Pure Rust (ngx-rust) Approach

1. **Request arrives** → Nginx calls Rust handler directly
2. **Rust handler** → Gets module configuration, verifies payment
3. **Payment verified** → Sends 402 response or allows request to proceed

**Status**: Core logic complete ✅, module registration framework ready ⚠️ (needs ngx-rust 0.5 API verification)

## CI/CD

This project uses GitHub Actions for continuous integration and automatic releases:

- **CI Workflow** (`.github/workflows/ci.yml`): Runs on every push and PR
  - Code formatting check (`cargo fmt`)
  - Linting (`cargo clippy`)
  - Unit tests (`cargo test`)
  - Build verification (Linux and macOS)

- **Release Workflow** (`.github/workflows/release.yml`): Runs on push to `master`
  - Automatically creates a release with tag format: `v{version}-{commit}`
  - Example: `v0.1.0-abc1234`
  - Builds and uploads artifacts for Linux and macOS

## License

AGPL-3.0
