#!/usr/bin/env bash

# Test runner script for nginx-x402 Test::Nginx tests
#
# This script sets up the environment and runs Test::Nginx tests.
#
# Usage:
#   ./go.sh t/basic.t
#   ./go.sh t/*.t
#   ./go.sh -v t/basic.t  # verbose output

set -e

# Find Nginx binary
if [ -z "$TEST_NGINX_BINARY" ]; then
    # Try common locations
    if command -v nginx >/dev/null 2>&1; then
        export TEST_NGINX_BINARY=$(which nginx)
    elif [ -f "/usr/local/nginx/sbin/nginx" ]; then
        export TEST_NGINX_BINARY="/usr/local/nginx/sbin/nginx"
    elif [ -f "/usr/local/openresty/nginx/sbin/nginx" ]; then
        export TEST_NGINX_BINARY="/usr/local/openresty/nginx/sbin/nginx"
    else
        echo "Error: nginx binary not found. Set TEST_NGINX_BINARY environment variable."
        exit 1
    fi
fi

# Set library path
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Find library (try macOS first, then Linux)
LIB_PATH_MAC="$SCRIPT_DIR/target/aarch64-apple-darwin/release"
LIB_PATH_LINUX="$SCRIPT_DIR/target/x86_64-unknown-linux-gnu/release"

if [ -f "$LIB_PATH_MAC/libnginx_x402.dylib" ]; then
    export DYLD_LIBRARY_PATH="$LIB_PATH_MAC:$DYLD_LIBRARY_PATH"
    export LD_LIBRARY_PATH="$LIB_PATH_MAC:$LD_LIBRARY_PATH"
    echo "Using library: $LIB_PATH_MAC/libnginx_x402.dylib"
elif [ -f "$LIB_PATH_LINUX/libnginx_x402.so" ]; then
    export LD_LIBRARY_PATH="$LIB_PATH_LINUX:$LD_LIBRARY_PATH"
    echo "Using library: $LIB_PATH_LINUX/libnginx_x402.so"
else
    # Try to find any built library
    FOUND_LIB=$(find "$SCRIPT_DIR/target" -name "libnginx_x402.*" -type f 2>/dev/null | head -1)
    if [ -n "$FOUND_LIB" ]; then
        LIB_DIR=$(dirname "$FOUND_LIB")
        if [[ "$FOUND_LIB" == *.dylib ]]; then
            export DYLD_LIBRARY_PATH="$LIB_DIR:$DYLD_LIBRARY_PATH"
        fi
        export LD_LIBRARY_PATH="$LIB_DIR:$LD_LIBRARY_PATH"
        echo "Using library: $FOUND_LIB"
    else
        echo "Warning: Library not found. Please build first: cargo build --release --features ngx-rust"
    fi
fi

# Note: For ngx-rust, the module is built as a dynamic library (.so/.dylib)
# and loaded directly by Nginx using load_module directive

# Set Perl library path
export PERL5LIB="$SCRIPT_DIR:$PERL5LIB"

# Run prove with arguments
exec prove "$@"

