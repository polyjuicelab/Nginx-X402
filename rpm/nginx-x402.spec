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
Requires:       cargo
Requires:       rustc
Requires:       clang-devel
Requires:       openssl-devel
Requires:       pkgconfig
Requires:       gcc
Requires:       make
Requires:       wget
Requires:       ca-certificates
Requires:       curl

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
# Don't build during package creation - build happens during installation (%post)
# This allows the module to be compiled against the actual system nginx version
echo "Skipping build during package creation - will build during installation"

%install
# Install source code and documentation - module will be built during installation
mkdir -p %{buildroot}%{moduledir}
mkdir -p %{buildroot}%{_docdir}/%{name}
mkdir -p %{buildroot}%{_sysconfdir}/nginx/modules-available
mkdir -p %{buildroot}%{_datadir}/%{name}/src

# Copy source code for building during installation
cp -r src %{buildroot}%{_datadir}/%{name}/
cp build.rs %{buildroot}%{_datadir}/%{name}/ 2>/dev/null || true
cp Cargo.toml %{buildroot}%{_datadir}/%{name}/
# Copy README.md (needed for doc generation in src/lib.rs)
cp README.md %{buildroot}%{_datadir}/%{name}/ 2>/dev/null || true
# Don't copy Cargo.lock - let cargo regenerate it during installation
# This avoids lock file version compatibility issues

# Install documentation
cp README.md %{buildroot}%{_docdir}/%{name}/
cp LICENSE %{buildroot}%{_docdir}/%{name}/
cp nginx/example.conf %{buildroot}%{_docdir}/%{name}/example.conf

# Create module configuration snippet
echo "load_module %{moduledir}/libnginx_x402.so;" > %{buildroot}%{_sysconfdir}/nginx/modules-available/x402.conf

# Create placeholder file for module (will be replaced in %post)
# This is needed because %files section requires the file to exist
touch %{buildroot}%{moduledir}/libnginx_x402.so

%post
#!/bin/bash
# Build the module during package installation to match system nginx version
SRC_DIR="%{_datadir}/%{name}"
BUILD_DIR="/tmp/%{name}-build"
MODULE_DIR="%{moduledir}"

echo "Building %{name} module for your system..."

# Detect nginx version
NGINX_VERSION=""
if command -v nginx >/dev/null 2>&1; then
    NGINX_VERSION=$(nginx -v 2>&1 | grep -oE 'nginx/[0-9]+\.[0-9]+\.[0-9]+' | cut -d'/' -f2 || echo "")
elif rpm -q nginx >/dev/null 2>&1; then
    NGINX_VERSION=$(rpm -q --qf '%%{VERSION}' nginx 2>/dev/null | cut -d'-' -f1 || echo "")
fi

if [ -z "$NGINX_VERSION" ]; then
    echo "ERROR: Could not detect nginx version. Please ensure nginx is installed."
    exit 1
fi

echo "Detected system nginx version: $NGINX_VERSION"

# Find or download matching nginx source
NGINX_SOURCE_DIR=""
if [ -d "/usr/src/nginx-$NGINX_VERSION" ] && [ -d "/usr/src/nginx-$NGINX_VERSION/objs" ]; then
    NGINX_SOURCE_DIR="/usr/src/nginx-$NGINX_VERSION"
elif [ -d "/usr/share/nginx-$NGINX_VERSION" ] && [ -d "/usr/share/nginx-$NGINX_VERSION/objs" ]; then
    NGINX_SOURCE_DIR="/usr/share/nginx-$NGINX_VERSION"
elif [ -d "/tmp/nginx-$NGINX_VERSION" ] && [ -d "/tmp/nginx-$NGINX_VERSION/objs" ]; then
    NGINX_SOURCE_DIR="/tmp/nginx-$NGINX_VERSION"
else
    echo "Downloading nginx-$NGINX_VERSION source..."
    mkdir -p /tmp
    if wget -q -O "/tmp/nginx-$NGINX_VERSION.tar.gz" "http://nginx.org/download/nginx-$NGINX_VERSION.tar.gz" 2>/dev/null; then
        (cd /tmp && tar -xzf "nginx-$NGINX_VERSION.tar.gz" && rm "nginx-$NGINX_VERSION.tar.gz")
        if [ -d "/tmp/nginx-$NGINX_VERSION" ]; then
            echo "Configuring nginx source..."
            (cd "/tmp/nginx-$NGINX_VERSION" && ./configure --without-http_rewrite_module >/dev/null 2>&1 || \
            ./configure --without-http_rewrite_module --with-cc-opt="-fPIC" >/dev/null 2>&1 || true)
            if [ -d "/tmp/nginx-$NGINX_VERSION/objs" ]; then
                NGINX_SOURCE_DIR="/tmp/nginx-$NGINX_VERSION"
            fi
        fi
    fi
fi

if [ -z "$NGINX_SOURCE_DIR" ] || [ ! -d "$NGINX_SOURCE_DIR/objs" ]; then
    echo "ERROR: Failed to find or configure nginx source for version $NGINX_VERSION"
    exit 1
fi

echo "Using nginx source: $NGINX_SOURCE_DIR"

# Set up build environment
export NGINX_SOURCE_DIR="$NGINX_SOURCE_DIR"
export CARGO_FEATURES="--no-default-features"
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"

# Set libclang path
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

# Verify Rust toolchain is available
if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: cargo not found. Please install cargo."
    exit 1
fi

if ! command -v rustc >/dev/null 2>&1; then
    echo "ERROR: rustc not found. Please install rustc."
    exit 1
fi

# Check Rust version - need at least 1.80.0 for edition2024 support
RUST_VERSION=$(rustc --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
RUST_MAJOR=$(echo "$RUST_VERSION" | cut -d'.' -f1)
RUST_MINOR=$(echo "$RUST_VERSION" | cut -d'.' -f2)

if [ "$RUST_MAJOR" -lt 1 ] || ([ "$RUST_MAJOR" -eq 1 ] && [ "$RUST_MINOR" -lt 80 ]); then
    echo "WARNING: System Rust version $RUST_VERSION is too old. Need at least 1.80.0"
    echo "Attempting to install rustup and use newer Rust version..."
    
    # Try to install rustup if not available
    if ! command -v rustup >/dev/null 2>&1; then
        echo "Installing rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable || {
            echo "ERROR: Failed to install rustup"
            echo "Please install rustup manually or update your system Rust package to at least 1.80.0"
            exit 1
        }
        # Source rustup environment
        export PATH="$HOME/.cargo/bin:$PATH"
        # Try to source rustup env, but don't fail if it doesn't exist
        if [ -f "$HOME/.cargo/env" ]; then
            . "$HOME/.cargo/env"
        fi
    fi
    
    # Use rustup to install/use stable toolchain
    if command -v rustup >/dev/null 2>&1; then
        echo "Installing stable Rust toolchain via rustup..."
        rustup toolchain install stable --profile minimal || {
            echo "ERROR: Failed to install Rust toolchain via rustup"
            exit 1
        }
        rustup default stable || {
            echo "ERROR: Failed to set default Rust toolchain"
            exit 1
        }
        export PATH="$HOME/.cargo/bin:$PATH"
        
        # Verify new version
        RUST_VERSION=$(rustc --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
        RUST_MAJOR=$(echo "$RUST_VERSION" | cut -d'.' -f1)
        RUST_MINOR=$(echo "$RUST_VERSION" | cut -d'.' -f2)
        
        if [ "$RUST_MAJOR" -lt 1 ] || ([ "$RUST_MAJOR" -eq 1 ] && [ "$RUST_MINOR" -lt 80 ]); then
            echo "ERROR: Installed Rust version $RUST_VERSION is still too old"
            exit 1
        fi
        echo "Using Rust version from rustup: $RUST_VERSION"
    else
        echo "ERROR: rustup not available and system Rust version is too old"
        exit 1
    fi
else
    echo "Using Rust version: $RUST_VERSION"
fi

# Build the module
echo "Building module..."
mkdir -p "$BUILD_DIR"
cp -r "$SRC_DIR"/* "$BUILD_DIR/"
# Copy README.md if it exists in source directory (needed for doc generation)
if [ -f "$SRC_DIR/README.md" ]; then
    cp "$SRC_DIR/README.md" "$BUILD_DIR/"
fi
cd "$BUILD_DIR"

# Ensure Cargo.toml exists
if [ ! -f "Cargo.toml" ]; then
    echo "ERROR: Cargo.toml not found in source directory"
    rm -rf "$BUILD_DIR"
    exit 1
fi

# Remove Cargo.lock if it exists (let cargo regenerate it)
# This avoids lock file version compatibility issues
rm -f Cargo.lock

# Generate fresh Cargo.lock
echo "Generating Cargo.lock..."
cargo generate-lockfile || {
    echo "WARNING: Failed to generate Cargo.lock, continuing anyway..."
}

cargo build --release $CARGO_FEATURES || {
    echo "ERROR: Failed to build module"
    echo "Build logs:"
    cargo build --release $CARGO_FEATURES 2>&1 | tail -50
    rm -rf "$BUILD_DIR"
    exit 1
}

# Find and copy the built module
MODULE_FILE=$(find target -name "libnginx_x402.so" -type f | head -1)
if [ -z "$MODULE_FILE" ]; then
    echo "ERROR: Built module not found"
    rm -rf "$BUILD_DIR"
    exit 1
fi

# Ensure module directory exists
mkdir -p "$MODULE_DIR"
if [ ! -d "$MODULE_DIR" ]; then
    echo "ERROR: Failed to create module directory: $MODULE_DIR"
    rm -rf "$BUILD_DIR"
    exit 1
fi

cp "$MODULE_FILE" "$MODULE_DIR/libnginx_x402.so"
chmod 644 "$MODULE_DIR/libnginx_x402.so"

# Clean up build directory
rm -rf "$BUILD_DIR"

echo "Module built and installed successfully!"
echo "Module location: $MODULE_DIR/libnginx_x402.so"

%preun
# Clean up module file before removal
MODULE_FILE="%{moduledir}/libnginx_x402.so"
if [ -f "$MODULE_FILE" ]; then
    rm -f "$MODULE_FILE"
fi

%files
%defattr(-,root,root,-)
%ghost %{moduledir}/libnginx_x402.so
%config(noreplace) %{_sysconfdir}/nginx/modules-available/x402.conf
%doc %{_docdir}/%{name}/README.md
%doc %{_docdir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/example.conf
%{_datadir}/%{name}/

%changelog
* Mon Nov 24 2025 Ryan Kung <ryan@polyjuice.io> - 1.0.0-1
- Implement custom auto-detect nginx version functionality
- Automatically downloads matching nginx source during build
- Removed vendored feature dependency
- Ensures module version matches system nginx version
