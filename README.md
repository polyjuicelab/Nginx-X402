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

## Environment Variables

### Required (when not using `vendored` feature)

- **`NGINX_SOURCE_DIR`**: Path to Nginx source code directory
  ```bash
  export NGINX_SOURCE_DIR=/path/to/nginx-1.29.1
  ```
  - Use the same Nginx version as your production environment
  - Required if `vendored` feature is not enabled

- **`NGINX_BUILD_DIR`** (optional): Path to Nginx build directory
  ```bash
  export NGINX_BUILD_DIR=/path/to/nginx-build
  ```
  - Alternative to `NGINX_SOURCE_DIR` if you have a pre-built Nginx

### Optional (for libclang)

- **`LIBCLANG_PATH`**: Path to libclang library (macOS only, usually not needed)
  ```bash
  # macOS with Homebrew
  export LIBCLANG_PATH=$(brew --prefix llvm)/lib
  
  # macOS with Xcode
  export LIBCLANG_PATH=$(xcode-select --print-path)/Toolchains/XcodeDefault.xctoolchain/usr/lib
  
  # Linux (usually not needed, system libclang is used)
  export LIBCLANG_PATH=/usr/lib/llvm-*/lib
  ```
  - Only needed if bindgen cannot find libclang automatically
  - On Linux, install `libclang-dev` package: `sudo apt-get install libclang-dev`
  - On macOS, install via Homebrew: `brew install llvm`

### Optional (for PCRE when using vendored feature)

When using the `vendored` feature, Nginx source compilation requires PCRE library. If you encounter PCRE-related errors:

- **`PKG_CONFIG_PATH`**: Help pkg-config find PCRE (macOS with Homebrew)
  ```bash
  # macOS with Homebrew
  export PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig:$PKG_CONFIG_PATH
  
  # Or for Intel Mac
  export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH
  ```

- **Install PCRE** (if not already installed):
  ```bash
  # macOS
  brew install pcre
  
  # Linux
  sudo apt-get install libpcre3-dev
  ```

- **`NGX_CONFIGURE_ARGS`**: Specify PCRE path directly
  ```bash
  # Option 1: Use PCRE source code (recommended)
  # Download PCRE source and extract it, then:
  export NGX_CONFIGURE_ARGS="--with-pcre=/path/to/pcre-8.45"
  
  # Option 2: Use pre-built PCRE library (macOS with Homebrew)
  # Note: This requires PCRE source code, not just the library
  # You may need to download PCRE source separately:
  # wget https://sourceforge.net/projects/pcre/files/pcre/8.45/pcre-8.45.tar.gz
  # tar -xzf pcre-8.45.tar.gz
  export NGX_CONFIGURE_ARGS="--with-pcre=/path/to/pcre-8.45"
  
  # Option 3: Disable HTTP rewrite module (not recommended)
  export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
  ```
  - Nginx's `--with-pcre` requires PCRE **source code**, not just the compiled library
  - Homebrew installs only the compiled library, so you need to download PCRE source separately
  - Download PCRE from: https://sourceforge.net/projects/pcre/files/pcre/

### Example: Complete Setup

**Linux:**
```bash
# Install system dependencies
sudo apt-get update
sudo apt-get install -y libclang-dev libpcre3-dev

# Set Nginx source (if not using vendored)
export NGINX_SOURCE_DIR=/path/to/nginx-1.29.1

# Build
cargo build --release
```

**macOS:**
```bash
# Install system dependencies
brew install llvm pcre

# Set libclang path (if needed)
export LIBCLANG_PATH=$(brew --prefix llvm)/lib

# Set PKG_CONFIG_PATH for PCRE (if using vendored feature)
export PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig:$PKG_CONFIG_PATH

# Set Nginx source (if not using vendored)
export NGINX_SOURCE_DIR=/path/to/nginx-1.29.1

# Build
cargo build --release
```

**Using vendored feature:**
```bash
# Install PCRE library (for linking)
# macOS:
brew install pcre

# Linux:
sudo apt-get install libpcre3-dev

# Download PCRE source code (required by Nginx configure)
# Nginx's --with-pcre requires PCRE source, not just the library
cd /tmp
wget https://sourceforge.net/projects/pcre/files/pcre/8.45/pcre-8.45.tar.gz
tar -xzf pcre-8.45.tar.gz

# Set NGX_CONFIGURE_ARGS to point to PCRE source
export NGX_CONFIGURE_ARGS="--with-pcre=/tmp/pcre-8.45"

# Build
cargo build --release --features vendored
```

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
