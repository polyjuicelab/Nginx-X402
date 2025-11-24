%define modulename nginx-x402
%define moduledir %{_libdir}/nginx/modules

Name:           nginx-x402
Version:        1.0.0
Release:        1%{?dist}
Summary:        Pure Rust Nginx module for x402 HTTP micropayment protocol
License:        AGPL-3.0
URL:            https://github.com/polyjuicelab/Nginx-X402
Source0:        %{name}-%{version}.tar.gz
BuildArch:      %{_arch}

BuildRequires:  cargo
BuildRequires:  rustc
BuildRequires:  openssl-devel
BuildRequires:  pkgconfig
BuildRequires:  clang-devel
BuildRequires:  wget
BuildRequires:  gcc
BuildRequires:  make

Requires:       nginx >= 1.18.0

%description
Pure Rust implementation of Nginx module for x402 HTTP micropayment protocol.

This package provides a pure Rust implementation of an Nginx module
that implements the x402 HTTP micropayment protocol.

The module can be loaded dynamically by Nginx and provides payment
verification functionality for HTTP requests.

Features:
 - Pure Rust implementation using ngx-rust
 - Type-safe Nginx API bindings
 - Payment verification and 402 response handling
 - Configurable facilitator service integration

%prep
%setup -q

%build
# Auto-detect system nginx version and download matching source
# This implements our own "vendored" functionality that matches system nginx version
NGINX_VERSION=""
NGINX_SOURCE_DIR=""

# Try to detect nginx version from installed package
if command -v nginx >/dev/null 2>&1; then
    NGINX_VERSION=$(nginx -v 2>&1 | grep -oE 'nginx/[0-9]+\.[0-9]+\.[0-9]+' | cut -d'/' -f2 || echo "")
elif rpm -q nginx >/dev/null 2>&1; then
    NGINX_VERSION=$(rpm -q --qf '%{VERSION}' nginx 2>/dev/null | cut -d'-' -f1 || echo "")
fi

# If nginx version detected, find or download matching source
if [ -n "$NGINX_VERSION" ]; then
    echo "Detected system nginx version: $NGINX_VERSION"
    if [ -d /usr/src/nginx-$NGINX_VERSION ] && [ -d /usr/src/nginx-$NGINX_VERSION/objs ]; then
        NGINX_SOURCE_DIR=/usr/src/nginx-$NGINX_VERSION
        echo "Using nginx source from /usr/src/nginx-$NGINX_VERSION"
    elif [ -d /usr/share/nginx-$NGINX_VERSION ] && [ -d /usr/share/nginx-$NGINX_VERSION/objs ]; then
        NGINX_SOURCE_DIR=/usr/share/nginx-$NGINX_VERSION
        echo "Using nginx source from /usr/share/nginx-$NGINX_VERSION"
    elif [ -d /tmp/nginx-$NGINX_VERSION ] && [ -d /tmp/nginx-$NGINX_VERSION/objs ]; then
        NGINX_SOURCE_DIR=/tmp/nginx-$NGINX_VERSION
        echo "Using cached nginx source from /tmp/nginx-$NGINX_VERSION"
    else
        echo "System nginx source not found, attempting to download nginx-$NGINX_VERSION..."
        if wget -q -O /tmp/nginx-$NGINX_VERSION.tar.gz "http://nginx.org/download/nginx-$NGINX_VERSION.tar.gz" 2>/dev/null; then
            (cd /tmp && tar -xzf nginx-$NGINX_VERSION.tar.gz && rm nginx-$NGINX_VERSION.tar.gz)
            if [ -d /tmp/nginx-$NGINX_VERSION ]; then
                echo "Downloaded nginx-$NGINX_VERSION source, configuring..."
                (cd /tmp/nginx-$NGINX_VERSION && ./configure --without-http_rewrite_module >/dev/null 2>&1 || \
                ./configure --without-http_rewrite_module --with-cc-opt="-fPIC" >/dev/null 2>&1 || true)
                if [ -d /tmp/nginx-$NGINX_VERSION/objs ]; then
                    NGINX_SOURCE_DIR=/tmp/nginx-$NGINX_VERSION
                    echo "Successfully configured nginx-$NGINX_VERSION source"
                else
                    echo "ERROR: Failed to configure nginx source"
                    exit 1
                fi
            else
                echo "ERROR: Failed to extract nginx source"
                exit 1
            fi
        else
            echo "ERROR: Failed to download nginx source"
            exit 1
        fi
    fi
else
    echo "ERROR: Could not detect nginx version. Please ensure nginx is installed."
    exit 1
fi

# Set environment variables - we always use NGINX_SOURCE_DIR (no vendored feature)
if [ -n "$NGINX_SOURCE_DIR" ]; then
    export NGINX_SOURCE_DIR=$NGINX_SOURCE_DIR
    export CARGO_FEATURES="--no-default-features"
    echo "Building with nginx source: $NGINX_SOURCE_DIR"
else
    echo "ERROR: NGINX_SOURCE_DIR is not set"
    exit 1
fi

# Set libclang path if available
if [ -z "$LIBCLANG_PATH" ]; then
    if [ -d /usr/lib64/llvm/lib ]; then
        export LIBCLANG_PATH=/usr/lib64/llvm/lib
    elif [ -d /usr/lib/llvm/lib ]; then
        export LIBCLANG_PATH=/usr/lib/llvm/lib
    elif ls -d /usr/lib64/llvm*/lib >/dev/null 2>&1; then
        export LIBCLANG_PATH=$(ls -d /usr/lib64/llvm*/lib 2>/dev/null | head -1)
    elif ls -d /usr/lib/llvm-*/lib >/dev/null 2>&1; then
        export LIBCLANG_PATH=$(ls -d /usr/lib/llvm-*/lib 2>/dev/null | head -1)
    fi
fi

# Set NGX_CONFIGURE_ARGS if not already set
if [ -z "$NGX_CONFIGURE_ARGS" ]; then
    export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
fi

# Build the module
cargo build --release $CARGO_FEATURES

%install
# Create directories
mkdir -p %{buildroot}%{moduledir}
mkdir -p %{buildroot}%{_docdir}/%{name}
mkdir -p %{buildroot}%{_sysconfdir}/nginx/modules-available

# Copy the built module (try multiple locations for cross-compilation support)
find target -name "libnginx_x402.so" -type f | head -1 | xargs -I {} cp {} %{buildroot}%{moduledir}/libnginx_x402.so || \
cp target/release/libnginx_x402.so %{buildroot}%{moduledir}/libnginx_x402.so || \
cp target/*/release/libnginx_x402.so %{buildroot}%{moduledir}/libnginx_x402.so

# Install documentation
cp README.md %{buildroot}%{_docdir}/%{name}/
cp LICENSE %{buildroot}%{_docdir}/%{name}/
cp nginx/example.conf %{buildroot}%{_docdir}/%{name}/example.conf

# Create module configuration snippet
echo "load_module %{moduledir}/libnginx_x402.so;" > %{buildroot}%{_sysconfdir}/nginx/modules-available/x402.conf

%files
%{moduledir}/libnginx_x402.so
%config(noreplace) %{_sysconfdir}/nginx/modules-available/x402.conf
%doc %{_docdir}/%{name}/README.md
%doc %{_docdir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/example.conf

%changelog
* Mon Nov 24 2025 Ryan Kung <ryan@polyjuice.io> - 1.0.0-1
- Implement custom auto-detect nginx version functionality
- Automatically downloads matching nginx source during build
- Removed vendored feature dependency
- Ensures module version matches system nginx version
