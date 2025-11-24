# nginx-x402

<div align="center">
  <img src="logo.png" alt="nginx-x402 logo" width="200">
</div>

<div align="center">

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](https://opensource.org/licenses/AGPL-3.0)
[![CI](https://github.com/polyjuicelab/Nginx-X402/actions/workflows/ci.yml/badge.svg)](https://github.com/polyjuicelab/Nginx-X402/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/nginx-x402.svg)](https://crates.io/crates/nginx-x402)
[![Crates.io](https://img.shields.io/crates/d/nginx-x402.svg)](https://crates.io/crates/nginx-x402)
[![Rust](https://img.shields.io/badge/rust-1.81%2B-orange.svg)](https://www.rust-lang.org/)
[![GitHub release](https://img.shields.io/github/release/polyjuicelab/Nginx-X402.svg)](https://github.com/polyjuicelab/Nginx-X402/releases)

</div>

Pure Rust implementation of Nginx module for x402 HTTP micropayment protocol.

## Overview

This project provides a **pure Rust** Nginx module that can be loaded directly by Nginx. The module implements the x402 payment verification protocol using the official [ngx-rust](https://github.com/nginx/ngx-rust) crate.

## Architecture

Uses the official [ngx-rust](https://github.com/nginx/ngx-rust) crate to implement the module entirely in Rust. **No C wrapper needed!**

- ✅ 100% Rust implementation
- ✅ Type-safe Nginx API bindings
- ✅ Official Nginx support
- ✅ **Auto-detects system nginx version and downloads matching source** (in build scripts)

## Prerequisites

- Rust toolchain (latest stable, 1.81.0+)
- `rust-x402` library (from crates.io)
- **Automatic**: Build scripts automatically detect system nginx version and download matching source
  - No manual nginx source needed
  - Automatically matches production nginx version
  - Works in deb/rpm builds and manual builds
  - Falls back to manual `NGINX_SOURCE_DIR` if detection fails
- **Manual (Optional)**: Provide Nginx source code manually
  - Set `NGINX_SOURCE_DIR` environment variable
  - Use `cargo build --release --no-default-features`
- libclang (for bindgen)

## Environment Variables

### Optional (when not using vendored feature)

If you want to use a specific Nginx version matching production, disable the default `vendored` feature:

```bash
# Disable vendored feature and use your own Nginx source
cargo build --release --no-default-features
```

- **`NGINX_SOURCE_DIR`**: Path to Nginx source code directory
  ```bash
  export NGINX_SOURCE_DIR=/path/to/nginx-1.29.1
  ```
  - Use the same Nginx version as your production environment
  - Required if `vendored` feature is disabled

- **`NGINX_BUILD_DIR`** (optional): Path to Nginx build directory
  ```bash
  export NGINX_BUILD_DIR=/path/to/nginx-build
  ```
  - Alternative to `NGINX_SOURCE_DIR` if you have a pre-built Nginx

### Required (for macOS - vendored feature is enabled by default)

- **`SDKROOT`**: macOS SDK path (required for finding system headers like `sys/types.h`)
  ```bash
  export SDKROOT=$(xcrun --show-sdk-path)
  ```
  - Required on macOS when using `vendored` feature
  - Ensures system headers are found during Nginx source compilation

- **`LIBCLANG_PATH`**: Path to libclang library (required for bindgen on macOS)
  ```bash
  # macOS with Xcode (recommended)
  export LIBCLANG_PATH="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib"
  
  # macOS with Homebrew (alternative)
  export LIBCLANG_PATH=$(brew --prefix llvm)/lib
  
  # Linux (usually not needed, system libclang is used)
  export LIBCLANG_PATH=/usr/lib/llvm-*/lib
  ```
  - Required on macOS for bindgen to generate Rust FFI bindings
  - On Linux, install `libclang-dev` package: `sudo apt-get install libclang-dev`
  - Ensure Xcode Command Line Tools are installed: `xcode-select --install`

### Recommended (vendored feature enabled by default - disable rewrite module)

Since the `vendored` feature is enabled by default, the simplest approach is to disable the HTTP rewrite module (which requires PCRE). Since the x402 plugin does not use rewrite functionality, this is safe and recommended:

- **`NGX_CONFIGURE_ARGS`**: Disable HTTP rewrite module (recommended for plugin builds)
  ```bash
  export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
  ```
  - **Recommended**: Simplest approach, no PCRE dependency needed
  - Safe because the x402 plugin does not use HTTP rewrite functionality
  - Avoids the need to download PCRE source code

### Optional (if you need rewrite module)

If you need the HTTP rewrite module for other purposes (vendored feature is enabled by default):

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

- **`NGX_CONFIGURE_ARGS`**: Specify PCRE source code path
  ```bash
  # Download PCRE source code (required by Nginx configure)
  # Nginx's --with-pcre requires PCRE source, not just the compiled library
  cd /tmp
  wget https://sourceforge.net/projects/pcre/files/pcre/8.45/pcre-8.45.tar.gz
  tar -xzf pcre-8.45.tar.gz
  
  # Set NGX_CONFIGURE_ARGS to point to PCRE source
  export NGX_CONFIGURE_ARGS="--with-pcre=/tmp/pcre-8.45"
  ```
  - Nginx's `--with-pcre` requires PCRE **source code**, not just the compiled library
  - Homebrew installs only the compiled library, so you need to download PCRE source separately
  - Download PCRE from: <https://sourceforge.net/projects/pcre/files/pcre/>

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
# Ensure Xcode Command Line Tools are installed
xcode-select --install

# Set Nginx source (if not using vendored)
export NGINX_SOURCE_DIR=/path/to/nginx-1.29.1

# Set libclang path (if using vendored feature)
export LIBCLANG_PATH="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib"

# Set SDK path (if using vendored feature)
export SDKROOT=$(xcrun --show-sdk-path)

# Build
cargo build --release
```

**Using vendored feature (recommended for plugin builds):**

**macOS:**
```bash
# Ensure Xcode Command Line Tools are installed
xcode-select --install

# Set required environment variables
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
export SDKROOT=$(xcrun --show-sdk-path)
export LIBCLANG_PATH="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib"

# Build
cargo build --release
```

**Linux:**
```bash
# Install system dependencies
sudo apt-get install -y libclang-dev

# Disable rewrite module (simplest, no PCRE needed)
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"

# Build
cargo build --release
```

**Alternative (if you need rewrite module):**
```bash
# Install PCRE library
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
cargo build --release
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

**macOS:**
```bash
# Set required environment variables
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
export SDKROOT=$(xcrun --show-sdk-path)
export LIBCLANG_PATH="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib"

# Build (ngx-rust will download Nginx source automatically)
cargo build --release
```

**Linux:**
```bash
# Disable rewrite module (simplest, no PCRE needed)
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"

# Build (ngx-rust will download Nginx source automatically)
cargo build --release
```

**Note**: The `vendored` feature (enabled by default) downloads a default Nginx version. For production, disable vendored and use Method A with the exact Nginx version you'll deploy: `cargo build --release --no-default-features`

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
- `x402_metrics on|off` - Enable/disable Prometheus metrics endpoint (for `/metrics` location)

## Monitoring with Prometheus and Grafana

The module exposes Prometheus metrics for monitoring payment verification performance and health.

### Available Metrics

- `x402_requests_total` - Total number of requests processed by x402 module
- `x402_payment_verifications_total` - Total number of payment verifications attempted
- `x402_payment_verifications_success_total` - Total number of successful payment verifications
- `x402_payment_verifications_failed_total` - Total number of failed payment verifications
- `x402_responses_402_total` - Total number of 402 Payment Required responses sent
- `x402_facilitator_errors_total` - Total number of facilitator service errors
- `x402_verification_duration_seconds` - Payment verification duration histogram (in seconds)
- `x402_payment_amount` - Payment amount histogram (in USDC)

### Configuration

Add a `/metrics` location to your Nginx configuration:

```nginx
http {
    server {
        # Prometheus metrics endpoint
        location /metrics {
            x402_metrics on;
            access_log off;  # Optional: disable access logging
        }

        # Protected API endpoint
        location /api/protected {
            x402 on;
            x402_amount 0.0001;
            x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
            x402_facilitator_url https://x402.org/facilitator;
        }
    }
}
```

### Prometheus Configuration

Add the following to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'nginx-x402'
    static_configs:
      - targets: ['your-nginx-server:80']
    metrics_path: '/metrics'
```

### Grafana Dashboard

You can create Grafana dashboards to visualize:

- Request rate and payment verification success rate
- Payment verification latency (p50, p95, p99)
- Error rates (facilitator errors, failed verifications)
- Payment amount distribution
- 402 response rate

Example queries:

```promql
# Request rate
rate(x402_requests_total[5m])

# Success rate
rate(x402_payment_verifications_success_total[5m]) / rate(x402_payment_verifications_total[5m])

# Verification latency (p95)
histogram_quantile(0.95, rate(x402_verification_duration_seconds_bucket[5m]))

# Error rate
rate(x402_facilitator_errors_total[5m])
```

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
- **Metrics Tests** (`metrics_tests.rs`) - Prometheus metrics collection and export

All tests can run without requiring Nginx source code or a running Nginx instance.

## How It Works

### Pure Rust (ngx-rust) Approach

1. **Request arrives** → Nginx calls Rust handler directly
2. **Rust handler** → Gets module configuration, verifies payment
3. **Payment verified** → Sends 402 response or allows request to proceed

**Status**: Core logic complete ✅, module registration framework ready ⚠️ (needs ngx-rust 0.5 API verification)

## Installing on Debian/Ubuntu

### Method 1: Install from GitHub Release (Recommended)

Download the pre-built `.deb` package from [GitHub Releases](https://github.com/polyjuicelab/Nginx-X402/releases):

**Supported Architectures:**
- `amd64` (x86_64) - Intel/AMD 64-bit
- `arm64` (aarch64) - ARM 64-bit
- `armhf` (armv7) - ARM hard float

**Option A: Download latest version automatically (amd64)**

```bash
# Download the latest release for amd64 (automatically gets the newest version)
wget https://github.com/polyjuicelab/Nginx-X402/releases/download/latest/nginx-x402_latest_amd64.deb -O nginx-x402_latest_amd64.deb

# Install the package
sudo dpkg -i nginx-x402_latest_amd64.deb
```

**Option B: Download specific version and architecture**

```bash
# Determine your architecture
ARCH=$(dpkg --print-architecture)

# Download a specific version (replace VERSION and COMMIT with actual values)
wget https://github.com/polyjuicelab/Nginx-X402/releases/download/v<VERSION>-<COMMIT>/nginx-x402_<VERSION>-1_${ARCH}.deb

# Install the package
sudo dpkg -i nginx-x402_<VERSION>-1_${ARCH}.deb
```

**Option C: Use GitHub API to get latest release**

```bash
# Determine your architecture
ARCH=$(dpkg --print-architecture)

# Get latest release download URL automatically for your architecture
LATEST_URL=$(curl -s https://api.github.com/repos/polyjuicelab/Nginx-X402/releases/latest | grep "browser_download_url.*${ARCH}\.deb" | cut -d '"' -f 4 | head -1)
wget "$LATEST_URL" -O nginx-x402_latest_${ARCH}.deb

# Install the package
sudo dpkg -i nginx-x402_latest_${ARCH}.deb
```

After installation (for all options above):

```bash
# If there are dependency issues, fix them with:
sudo apt-get install -f

# Enable the module in Nginx
sudo ln -s /etc/nginx/modules-available/x402.conf /etc/nginx/modules-enabled/x402.conf

# Test Nginx configuration
sudo nginx -t

# Reload Nginx to load the module
sudo systemctl reload nginx
```

### Method 2: Build from Source

To build a Debian package (`.deb` file) from source:

#### Prerequisites

```bash
# Install build dependencies
sudo apt-get update
sudo apt-get install -y \
    debhelper \
    cargo \
    rustc \
    libssl-dev \
    pkg-config \
    libclang-dev \
    build-essential
```

#### Build the Package

```bash
# Clone the repository
git clone https://github.com/polyjuicelab/Nginx-X402.git
cd nginx-x402

# Build the deb package
dpkg-buildpackage -b -us -uc

# The resulting .deb file will be in the parent directory:
# ../nginx-x402_<VERSION>-1_amd64.deb
```

#### Install the Built Package

```bash
# Install the built package
sudo dpkg -i ../nginx-x402_<VERSION>-1_amd64.deb

# If there are dependency issues, fix them with:
sudo apt-get install -f

# Enable the module in Nginx
sudo ln -s /etc/nginx/modules-available/x402.conf /etc/nginx/modules-enabled/x402.conf

# Test Nginx configuration
sudo nginx -t

# Reload Nginx to load the module
sudo systemctl reload nginx
```

### Package Contents

The deb package installs:
- `/usr/lib/nginx/modules/libnginx_x402.so` - The Nginx module
- `/etc/nginx/modules-available/x402.conf` - Module load configuration snippet
- `/usr/share/doc/nginx-x402/` - Documentation and example configuration

### Configuration

After installation, add the module configuration to your `nginx.conf`:

```nginx
# Load the module (already done if you used the symlink above)
load_module /usr/lib/nginx/modules/libnginx_x402.so;

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

### Verification

Verify the module is loaded:

```bash
# Check if module is loaded
nginx -V 2>&1 | grep -i x402

# Or check Nginx error log for module loading messages
sudo tail -f /var/log/nginx/error.log
```

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
