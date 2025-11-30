#!/bin/bash
set -e

# Script to run clippy with proper nginx environment setup

cd "$(dirname "$0")/.."

# Use existing nginx source directory if available, otherwise create minimal one
NGX_VERSION="${NGX_VERSION:-1.28.0}"
NGINX_SOURCE_DIR="/tmp/nginx-${NGX_VERSION}"

# Check if nginx source is already configured
if [ ! -f "${NGINX_SOURCE_DIR}/objs/Makefile" ]; then
    echo "Nginx source not found, creating minimal setup for clippy..."
mkdir -p "$NGINX_SOURCE_DIR/objs"

# Create minimal ngx_auto_config.h
cat > "$NGINX_SOURCE_DIR/objs/ngx_auto_config.h" << 'EOF'
#define NGX_PTR_SIZE 8
#define NGX_SIG_ATOMIC_T_SIZE 4
#define NGX_TIME_T_SIZE 8
#define NGX_MODULE_SIGNATURE_1 "test"
EOF

# Create minimal Makefile
touch "$NGINX_SOURCE_DIR/objs/Makefile"
fi

export NGINX_SOURCE_DIR

# Set libclang path for bindgen
if [ -f "/Library/Developer/CommandLineTools/usr/lib/libclang.dylib" ]; then
    export LIBCLANG_PATH="/Library/Developer/CommandLineTools/usr/lib/libclang.dylib"
elif [ -f "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/libclang.dylib" ]; then
    export LIBCLANG_PATH="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/libclang.dylib"
fi

# Set SDK path for macOS
if command -v xcrun &> /dev/null; then
    SDK_PATH=$(xcrun --show-sdk-path 2>/dev/null)
    if [ -n "$SDK_PATH" ]; then
        export SDKROOT="$SDK_PATH"
        export BINDGEN_EXTRA_CLANG_ARGS="-isysroot $SDK_PATH -I$SDK_PATH/usr/include"
    fi
fi

# Check file sizes first
echo "Checking file sizes..."
if [ -f "scripts/check-file-size.sh" ]; then
    ./scripts/check-file-size.sh 500 src tests || {
        echo "❌ File size check failed"
        exit 1
    }
else
    echo "⚠️  check-file-size.sh not found, skipping file size check"
fi

echo "Running clippy with NGINX_SOURCE_DIR=$NGINX_SOURCE_DIR"
cargo clippy --all-targets --all-features -- -D warnings

echo "✅ Clippy passed!"

