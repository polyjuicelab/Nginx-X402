# nginx-x402

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

# Build with ngx-rust feature
cargo build --release --features ngx-rust

# The module will be at:
# target/aarch64-apple-darwin/release/libnginx_x402.dylib (macOS)
# target/x86_64-unknown-linux-gnu/release/libnginx_x402.so (Linux)
```

### Method B: Auto-download Nginx Source (Convenient for Development)

```bash
# No Nginx source needed! ngx-rust will download it automatically
cargo build --release --features ngx-rust-vendored
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
- `x402_testnet on|off` - Use testnet (default: on)
- `x402_description <text>` - Payment description
- `x402_network <network>` - Network identifier (e.g., "base-sepolia")
- `x402_resource <path>` - Resource path (default: request URI)

## Testing

### Rust Tests

```bash
# Run all tests
cargo test

# Run ngx-rust module tests
cargo test --test ngx_module_tests
```

The core payment verification logic is fully testable without requiring Nginx source code. See `tests/ngx_module_tests.rs` for examples.

### Test::Nginx Tests

```bash
# Build the library first
cargo build --release --features ngx-rust

# Run Test::Nginx tests (requires Nginx source)
./go.sh t/*.t
```

## How It Works

### Pure Rust (ngx-rust) Approach

1. **Request arrives** → Nginx calls Rust handler directly
2. **Rust handler** → Gets module configuration, verifies payment
3. **Payment verified** → Sends 402 response or allows request to proceed

**Status**: Core logic complete ✅, module registration framework ready ⚠️ (needs ngx-rust 0.5 API verification)

## License

AGPL-3.0
