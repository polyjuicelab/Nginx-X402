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

## Features

- ✅ 100% Rust implementation using [ngx-rust](https://github.com/nginx/ngx-rust)
- ✅ Auto-detects system nginx version and downloads matching source
- ✅ Type-safe Nginx API bindings
- ✅ Payment verification and 402 response handling
- ✅ Prometheus metrics support
- ✅ Custom token support with configurable decimals (ERC-20 compatible)
- ✅ Network identification via chainId (8453, 84532)
- ✅ Automatic amount calculation based on token decimals

## Quick Start

### Build from Source

**Linux:**
```bash
sudo apt-get install -y libclang-dev
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
cargo build --release
```

**macOS:**
```bash
xcode-select --install
eval $(./setup-build-env.sh)
cargo build --release
```

The build script automatically detects your system nginx version and downloads matching source. Optionally set `NGINX_SOURCE_DIR` to use a specific nginx source.

**macOS (Manual):**
```bash
export LIBCLANG_PATH="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib"
export SDKROOT=$(xcrun --show-sdk-path)
cargo build --release
```

### Install from Package (Debian/Ubuntu)

```bash
# Download latest release
ARCH=$(dpkg --print-architecture)
LATEST_URL=$(curl -s https://api.github.com/repos/polyjuicelab/Nginx-X402/releases/latest | grep "browser_download_url.*${ARCH}\.deb" | cut -d '"' -f 4 | head -1)
wget "$LATEST_URL" -O nginx-x402.deb

# Install
sudo dpkg -i nginx-x402.deb
sudo apt-get install -f
sudo ln -s /etc/nginx/modules-available/x402.conf /etc/nginx/modules-enabled/x402.conf
sudo systemctl reload nginx
```

## Configuration

**IMPORTANT: Module Loading**

You have two options to load the module. Choose **ONE** method only:

**Option 1: Using modules-enabled (Recommended for Debian/Ubuntu)**
```bash
sudo ln -s /etc/nginx/modules-available/x402.conf /etc/nginx/modules-enabled/x402.conf
```
This automatically loads the module via `/etc/nginx/modules-enabled/x402.conf`.

**Option 2: Manual load_module in nginx.conf**
Add this line to your `/etc/nginx/nginx.conf` (at the top level, before `http` block):
```nginx
load_module /usr/lib/nginx/modules/libnginx_x402.so;
```

**⚠️ DO NOT use both methods - this will cause "module already loaded" errors.**

After loading the module, add x402 configuration to your `nginx.conf`:

```nginx
http {
    server {
        # Basic USDC payment (default)
        location /protected {
            x402 on;
            x402_amount 0.0001;
            x402_pay_to 0xYourWalletAddress;
            x402_facilitator_url https://x402.org/facilitator;
            x402_network base-sepolia;
        }

        # Custom ERC-20 token with 18 decimals
        location /api/custom-token {
            x402 on;
            x402_amount 0.001;
            x402_pay_to 0xYourWalletAddress;
            x402_facilitator_url https://x402.org/facilitator;
            x402_network_id 84532;  # Base Sepolia chainId
            x402_asset 0xYourCustomTokenAddress;
            x402_asset_decimals 18;  # Standard ERC-20
        }
    }
}
```

### Configuration Directives

**Basic Configuration:**
- `x402 on|off` - Enable/disable payment verification
- `x402_amount <amount>` - Payment amount (e.g., "0.0001")
- `x402_pay_to <address>` - Recipient wallet address
- `x402_facilitator_url <url>` - Facilitator service URL
- `x402_description <text>` - Payment description

**Network Configuration:**
- `x402_network <network>` - Network identifier (e.g., "base", "base-sepolia")
- `x402_network_id <chainId>` - Network chainId (e.g., 8453 for Base Mainnet, 84532 for Base Sepolia). Takes precedence over `x402_network` if both are specified.

**Token Configuration:**
- `x402_asset <address>` - Custom token/contract address (optional, defaults to USDC for the network)
- `x402_asset_decimals <decimals>` - Token decimals (default: 6 for USDC, typically 18 for ERC-20 tokens). **Required when using custom tokens** to ensure correct amount calculation.

**Advanced Configuration:**
- `x402_resource <path>` - Resource path or full URL (default: automatically builds full URL from request: `scheme://host/path`)
- `x402_timeout <seconds>` - Facilitator API timeout (1-300, default: 10)
- `x402_facilitator_fallback <mode>` - Fallback on error: `error` (500) or `pass` (default: `error`)
- `x402_metrics on|off` - Enable Prometheus metrics endpoint

**Note:** If `x402_resource` is not configured, the module automatically builds a full URL from the request (`scheme://host/path`). This ensures compatibility with facilitator APIs that require full URLs instead of relative paths. If you need a relative path or custom URL, explicitly set `x402_resource`.

**Note:** When using custom tokens, always specify `x402_asset_decimals` to match your token's decimal precision. Most ERC-20 tokens use 18 decimals, while USDC uses 6 decimals.

## Monitoring

### Prometheus Metrics

Add a `/metrics` location:

```nginx
location /metrics {
    x402_metrics on;
    access_log off;
}
```

Available metrics:
- `x402_requests_total` - Total requests processed
- `x402_payment_verifications_total` - Verification attempts
- `x402_payment_verifications_success_total` - Successful verifications
- `x402_payment_verifications_failed_total` - Failed verifications
- `x402_responses_402_total` - 402 responses sent
- `x402_facilitator_errors_total` - Facilitator errors
- `x402_verification_duration_seconds` - Verification latency histogram
- `x402_payment_amount` - Payment amount histogram

### Prometheus Configuration

```yaml
scrape_configs:
  - job_name: 'nginx-x402'
    static_configs:
      - targets: ['your-nginx-server:80']
    metrics_path: '/metrics'
```

## Testing

**Unit Tests:**
```bash
cargo test
```

All unit tests run without requiring nginx source or a running nginx instance.

**Integration Tests:**
```bash
cargo test --test docker_integration_test --features integration-test -- --ignored
```

Integration tests require Docker and will automatically build and run nginx in a container.

## Environment Variables

- `NGINX_SOURCE_DIR` - Path to nginx source (optional, auto-detected if not set)
- `NGINX_BINARY_PATH` - Path to system nginx binary (optional, for signature extraction)
- `NGX_CONFIGURE_ARGS` - Nginx configure arguments (recommended: `--without-http_rewrite_module`)
- `LIBCLANG_PATH` - Path to libclang (required on macOS)
- `SDKROOT` - macOS SDK path (required on macOS)

## How It Works

1. Request arrives → Nginx calls Rust handler
2. Rust handler → Verifies payment via facilitator service
3. Payment verified → Allows request or sends 402 response

## License

AGPL-3.0
