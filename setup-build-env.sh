#!/bin/bash
# Setup build environment for nginx-x402 module
# Usage: source setup-build-env.sh
#   or: eval $(./setup-build-env.sh)

set -e

echo "Setting up build environment for nginx-x402..." >&2

# Detect nginx version
if command -v nginx >/dev/null 2>&1; then
    NGINX_VERSION=$(nginx -v 2>&1 | grep -oE 'nginx/[0-9]+\.[0-9]+\.[0-9]+' | cut -d'/' -f2)
    echo "Detected nginx version: $NGINX_VERSION" >&2
else
    echo "ERROR: nginx command not found" >&2
    echo "Please install nginx or set NGINX_VERSION manually" >&2
    exit 1
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

# Set macOS-specific environment variables
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [ -z "$LIBCLANG_PATH" ]; then
        if [ -d "$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib" ]; then
            export LIBCLANG_PATH="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib"
            echo "Set LIBCLANG_PATH=$LIBCLANG_PATH" >&2
        else
            echo "WARNING: Could not find libclang, bindgen may fail" >&2
        fi
    fi
    
    if [ -z "$SDKROOT" ]; then
        SDKROOT=$(xcrun --show-sdk-path 2>/dev/null || echo "")
        if [ -n "$SDKROOT" ]; then
            export SDKROOT
            echo "Set SDKROOT=$SDKROOT" >&2
        else
            echo "WARNING: Could not detect SDKROOT" >&2
        fi
    fi
fi

echo "" >&2
echo "Build environment ready:" >&2
echo "  NGINX_SOURCE_DIR=$NGINX_SOURCE_DIR" >&2
if [ -n "$LIBCLANG_PATH" ]; then
    echo "  LIBCLANG_PATH=$LIBCLANG_PATH" >&2
fi
if [ -n "$SDKROOT" ]; then
    echo "  SDKROOT=$SDKROOT" >&2
fi
echo "" >&2
echo "You can now run: cargo build --no-default-features" >&2

# Export environment variables for sourcing
echo "export NGINX_SOURCE_DIR=\"$NGINX_SOURCE_DIR\""
if [ -n "$LIBCLANG_PATH" ]; then
    echo "export LIBCLANG_PATH=\"$LIBCLANG_PATH\""
fi
if [ -n "$SDKROOT" ]; then
    echo "export SDKROOT=\"$SDKROOT\""
fi

