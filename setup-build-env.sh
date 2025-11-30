#!/bin/bash
# Setup build environment for nginx-x402 module
# Usage: source setup-build-env.sh
#   or: eval $(./setup-build-env.sh)
#
# This script sets up:
# - NGINX_SOURCE_DIR: Path to configured nginx source
# - CC: C compiler (clang or gcc)
# - LIBCLANG_PATH: Path to libclang (for bindgen on macOS)
# - SDKROOT: macOS SDK path (for bindgen on macOS)

set -e

echo "Setting up build environment for nginx-x402..." >&2

# Detect nginx version
if [ -n "$NGX_VERSION" ]; then
    NGINX_VERSION="$NGX_VERSION"
    echo "Using NGX_VERSION from environment: $NGINX_VERSION" >&2
elif [ -n "$NGINX_VERSION" ]; then
    NGINX_VERSION="$NGINX_VERSION"
    echo "Using NGINX_VERSION from environment: $NGINX_VERSION" >&2
elif command -v nginx >/dev/null 2>&1; then
    NGINX_VERSION=$(nginx -v 2>&1 | grep -oE 'nginx/[0-9]+\.[0-9]+\.[0-9]+' | cut -d'/' -f2)
    echo "Detected nginx version: $NGINX_VERSION" >&2
else
    echo "WARNING: nginx command not found, using default version 1.22.0" >&2
    NGINX_VERSION="1.22.0"
fi

# Download and configure nginx source if needed
if [ -z "$NGINX_SOURCE_DIR" ]; then
    NGINX_SOURCE_DIR="/tmp/nginx-$NGINX_VERSION"
    
    if [ ! -d "$NGINX_SOURCE_DIR/objs" ]; then
        echo "Downloading nginx-$NGINX_VERSION source..."
        cd /tmp
        
        if [ ! -f "nginx-$NGINX_VERSION.tar.gz" ]; then
            if command -v wget >/dev/null 2>&1; then
                wget -q "https://nginx.org/download/nginx-$NGINX_VERSION.tar.gz"
            elif command -v curl >/dev/null 2>&1; then
                curl -sSfL -o "nginx-$NGINX_VERSION.tar.gz" "https://nginx.org/download/nginx-$NGINX_VERSION.tar.gz"
            else
            echo "ERROR: wget or curl not found" >&2
            exit 1
            fi
        fi
        
        if [ ! -d "nginx-$NGINX_VERSION" ]; then
            tar -xzf "nginx-$NGINX_VERSION.tar.gz"
            rm -f "nginx-$NGINX_VERSION.tar.gz"
        fi
        
        cd "nginx-$NGINX_VERSION"
        echo "Configuring nginx source..."
        ./configure --without-http_rewrite_module --with-cc-opt="-fPIC" >/dev/null 2>&1 || {
            echo "Failed to configure nginx source" >&2
            exit 1
        }
        echo "Configured nginx source" >&2
    else
        echo "Using existing nginx source at $NGINX_SOURCE_DIR" >&2
    fi
    
    export NGINX_SOURCE_DIR
fi

# Detect and set C compiler
if [ -z "$CC" ]; then
    # Prefer clang if available, fallback to gcc
    if command -v clang >/dev/null 2>&1; then
        export CC=clang
        echo "Set CC=clang" >&2
    elif command -v gcc >/dev/null 2>&1; then
        export CC=gcc
        echo "Set CC=gcc" >&2
    else
        echo "WARNING: No C compiler found (clang or gcc)" >&2
        echo "WARNING: Build may fail without a C compiler" >&2
    fi
else
    echo "Using CC from environment: $CC" >&2
fi

# Set macOS-specific environment variables
if [[ "$OSTYPE" == "darwin"* ]]; then
    # Set LIBCLANG_PATH for bindgen
    if [ -z "$LIBCLANG_PATH" ]; then
        # Try multiple common locations
        LIBCLANG_PATHS=(
            "$(xcode-select -p 2>/dev/null)/Toolchains/XcodeDefault.xctoolchain/usr/lib"
            "/Library/Developer/CommandLineTools/usr/lib"
            "/usr/local/lib"
            "/opt/homebrew/lib"
        )
        
        for path in "${LIBCLANG_PATHS[@]}"; do
            if [ -d "$path" ] && [ -f "$path/libclang.dylib" ] || [ -f "$path/libclang.so" ]; then
                export LIBCLANG_PATH="$path"
                echo "Set LIBCLANG_PATH=$LIBCLANG_PATH" >&2
                break
            fi
        done
        
        if [ -z "$LIBCLANG_PATH" ]; then
            echo "WARNING: Could not find libclang, bindgen may fail" >&2
            echo "WARNING: Try: xcode-select --install" >&2
        fi
    else
        echo "Using LIBCLANG_PATH from environment: $LIBCLANG_PATH" >&2
    fi
    
    # Set SDKROOT for bindgen
    if [ -z "$SDKROOT" ]; then
        SDKROOT=$(xcrun --show-sdk-path 2>/dev/null || echo "")
        if [ -n "$SDKROOT" ]; then
            export SDKROOT
            echo "Set SDKROOT=$SDKROOT" >&2
        else
            echo "WARNING: Could not detect SDKROOT" >&2
            echo "WARNING: Try: xcode-select --install" >&2
        fi
    else
        echo "Using SDKROOT from environment: $SDKROOT" >&2
    fi
fi

# Set Linux-specific environment variables
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Ensure libclang-dev is available (for bindgen)
    if [ -z "$LIBCLANG_PATH" ]; then
        # Common locations for libclang on Linux
        LIBCLANG_PATHS=(
            "/usr/lib/llvm-*/lib"
            "/usr/lib/x86_64-linux-gnu"
            "/usr/lib64"
            "/usr/local/lib"
        )
        
        for pattern in "${LIBCLANG_PATHS[@]}"; do
            for path in $pattern; do
                if [ -d "$path" ] && [ -f "$path/libclang.so"* ]; then
                    export LIBCLANG_PATH="$path"
                    echo "Set LIBCLANG_PATH=$LIBCLANG_PATH" >&2
                    break 2
                fi
            done
        done
        
        if [ -z "$LIBCLANG_PATH" ]; then
            echo "WARNING: Could not find libclang, bindgen may fail" >&2
            echo "WARNING: Try: sudo apt-get install libclang-dev (Debian/Ubuntu)" >&2
            echo "WARNING:   or: sudo yum install clang-devel (RHEL/CentOS)" >&2
        fi
    else
        echo "Using LIBCLANG_PATH from environment: $LIBCLANG_PATH" >&2
    fi
fi

echo "" >&2
echo "Build environment ready:" >&2
echo "  NGINX_SOURCE_DIR=$NGINX_SOURCE_DIR" >&2
if [ -n "$CC" ]; then
    echo "  CC=$CC" >&2
fi
if [ -n "$LIBCLANG_PATH" ]; then
    echo "  LIBCLANG_PATH=$LIBCLANG_PATH" >&2
fi
if [ -n "$SDKROOT" ]; then
    echo "  SDKROOT=$SDKROOT" >&2
fi
echo "" >&2
echo "You can now run:" >&2
echo "  cargo build --release" >&2
echo "  cargo test --test docker_integration_test --features integration-test" >&2

# Export environment variables for sourcing
echo "export NGINX_SOURCE_DIR=\"$NGINX_SOURCE_DIR\""
if [ -n "$CC" ]; then
    echo "export CC=\"$CC\""
fi
if [ -n "$LIBCLANG_PATH" ]; then
    echo "export LIBCLANG_PATH=\"$LIBCLANG_PATH\""
fi
if [ -n "$SDKROOT" ]; then
    echo "export SDKROOT=\"$SDKROOT\""
fi

