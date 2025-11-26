%define modulename nginx-x402
%define moduledir %{_libdir}/nginx/modules

Name:           nginx-x402
Version:        1.0.4
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

# Detect nginx version - try multiple methods for accuracy
NGINX_VERSION=""
NGINX_VERSION_FULL=""

# Method 1: Use nginx -V to get detailed version info
if command -v nginx >/dev/null 2>&1; then
    NGINX_VERSION_FULL=$(nginx -v 2>&1)
    echo "Nginx version output: $NGINX_VERSION_FULL"
    NGINX_VERSION=$(echo "$NGINX_VERSION_FULL" | sed -n 's/.*nginx\/\([0-9]\+\.[0-9]\+\.[0-9]\+\).*/\1/p' | head -1)
    
    # If still not found, try -V output
    if [ -z "$NGINX_VERSION" ]; then
        NGINX_VERSION=$(nginx -V 2>&1 | sed -n 's/.*nginx\/\([0-9]\+\.[0-9]\+\.[0-9]\+\).*/\1/p' | head -1)
    fi
fi

# Method 2: Check rpm if nginx command didn't work
if [ -z "$NGINX_VERSION" ] && rpm -q nginx >/dev/null 2>&1; then
    NGINX_VERSION=$(rpm -q --qf '%%{VERSION}' nginx 2>/dev/null | cut -d'-' -f1 || echo "")
    echo "Detected nginx version from rpm: $NGINX_VERSION"
fi

# Method 3: Try to read from nginx binary directly
if [ -z "$NGINX_VERSION" ] && [ -f "/usr/sbin/nginx" ]; then
    NGINX_VERSION=$(strings /usr/sbin/nginx 2>/dev/null | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || echo "")
    echo "Detected nginx version from binary: $NGINX_VERSION"
fi

if [ -z "$NGINX_VERSION" ]; then
    echo "ERROR: Could not detect nginx version. Please ensure nginx is installed."
    echo "Tried methods: nginx -v, rpm -q, reading binary"
    exit 1
fi

echo "Detected system nginx version: $NGINX_VERSION"

# Find or download matching nginx source
# Check for configured nginx source (must have objs/ngx_modules.c)
NGINX_SOURCE_DIR=""
if [ -f "/usr/src/nginx-$NGINX_VERSION/objs/ngx_modules.c" ]; then
    NGINX_SOURCE_DIR="/usr/src/nginx-$NGINX_VERSION"
elif [ -f "/usr/share/nginx-$NGINX_VERSION/objs/ngx_modules.c" ]; then
    NGINX_SOURCE_DIR="/usr/share/nginx-$NGINX_VERSION"
elif [ -f "/tmp/nginx-$NGINX_VERSION/objs/ngx_modules.c" ]; then
    NGINX_SOURCE_DIR="/tmp/nginx-$NGINX_VERSION"
else
    echo "Downloading nginx-$NGINX_VERSION source..."
    mkdir -p /tmp
    # Try wget first, then curl (same as build.rs)
    DOWNLOAD_SUCCESS=0
    if command -v wget >/dev/null 2>&1 && wget -q -O "/tmp/nginx-$NGINX_VERSION.tar.gz" "https://nginx.org/download/nginx-$NGINX_VERSION.tar.gz" 2>/dev/null; then
        DOWNLOAD_SUCCESS=1
    elif command -v curl >/dev/null 2>&1 && curl -sSfL -o "/tmp/nginx-$NGINX_VERSION.tar.gz" "https://nginx.org/download/nginx-$NGINX_VERSION.tar.gz" 2>/dev/null; then
        DOWNLOAD_SUCCESS=1
    fi
    
    if [ "$DOWNLOAD_SUCCESS" -eq 1 ]; then
        (cd /tmp && tar -xzf "nginx-$NGINX_VERSION.tar.gz" && rm "nginx-$NGINX_VERSION.tar.gz")
        if [ -d "/tmp/nginx-$NGINX_VERSION" ]; then
            echo "Configuring nginx source..."
            # Get nginx configure arguments from system nginx
            NGINX_CONFIGURE_ARGS_SYSTEM=""
            if command -v nginx >/dev/null 2>&1; then
                NGINX_CONFIGURE_ARGS_SYSTEM=$(nginx -V 2>&1 | grep -oE 'configure arguments:.*' | sed 's/configure arguments://' || echo "")
                echo "System nginx configure args: $NGINX_CONFIGURE_ARGS_SYSTEM"
            fi
            
            # Configure nginx source with similar arguments
            if [ -n "$NGINX_CONFIGURE_ARGS_SYSTEM" ]; then
                # IMPORTANT: Only remove modules that cause build failures
                # DO NOT remove --with-cc-opt, --with-ld-opt, --with-debug, or other flags
                # that affect module signature and binary compatibility
                
                # Remove problematic modules that require additional dependencies
                # Remove rewrite module (we don't need it)
                CONFIGURE_ARGS_CLEAN=$(echo "$NGINX_CONFIGURE_ARGS_SYSTEM" | sed 's/--with-http_rewrite_module//g')
                # Remove dynamic modules that may require additional libraries
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/--with-http_xslt_module=dynamic//g')
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/--with-http_perl_module=dynamic//g')
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/--with-http_image_filter_module=dynamic//g')
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/--with-http_geoip_module=dynamic//g')
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/--with-mail=dynamic//g')
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/--with-stream=dynamic//g')
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/--with-stream_geoip_module=dynamic//g')
                
                # CRITICAL: Preserve all flags that affect module signature:
                # - --with-cc-opt=* (compiler options)
                # - --with-ld-opt=* (linker options)
                # - --with-debug (debug mode)
                # - --with-compat (compatibility mode)
                # - Any other flags that affect binary compatibility
                
                # Clean up multiple spaces
                CONFIGURE_ARGS_CLEAN=$(echo "$CONFIGURE_ARGS_CLEAN" | sed 's/  */ /g' | sed 's/^ *//' | sed 's/ *$//')
                
                # Add --without-http_rewrite_module if not already present
                if echo "$CONFIGURE_ARGS_CLEAN" | grep -qv -- "--without-http_rewrite_module"; then
                    CONFIGURE_ARGS_CLEAN="$CONFIGURE_ARGS_CLEAN --without-http_rewrite_module"
                fi
                
                # Debug: Show what flags are preserved
                echo "Preserved configure flags:"
                echo "$CONFIGURE_ARGS_CLEAN" | grep -oE '--with-(cc-opt|ld-opt|debug|compat)(=[^ ]*)?' || echo "  (none found)"
                
                echo "Running configure with system arguments..."
                echo "Cleaned configure args: $CONFIGURE_ARGS_CLEAN"
                echo ""
                echo "=== Configuration Comparison ==="
                echo "System nginx configure args (original):"
                echo "$NGINX_CONFIGURE_ARGS_SYSTEM"
                echo ""
                echo "Cleaned configure args (for build):"
                echo "$CONFIGURE_ARGS_CLEAN"
                echo ""
                echo "Removed modules (should not affect signature):"
                echo "  - --with-http_rewrite_module"
                echo "  - --with-http_xslt_module=dynamic"
                echo "  - --with-http_perl_module=dynamic"
                echo "  - --with-http_image_filter_module=dynamic"
                echo "  - --with-http_geoip_module=dynamic"
                echo "  - --with-mail=dynamic"
                echo "  - --with-stream=dynamic"
                echo "  - --with-stream_geoip_module=dynamic"
                echo ""
                echo "Preserved flags (critical for signature):"
                echo "$CONFIGURE_ARGS_CLEAN" | grep -oE '--with-(cc-opt|ld-opt|debug|compat|threads|file-aio|http_ssl_module|http_realip_module|http_gzip_static_module)(=[^ ]*)?' || echo "  (checking...)"

                # Use sh -c to properly handle quoted arguments (same as build.rs)
                (cd "/tmp/nginx-$NGINX_VERSION" && sh -c "./configure $CONFIGURE_ARGS_CLEAN" >/tmp/nginx-configure.log 2>&1 || {
                    echo "Configure with system args failed, checking log..."
                    if [ -f /tmp/nginx-configure.log ]; then
                        echo "Last 50 lines of configure log:"
                        tail -50 /tmp/nginx-configure.log || true
                    fi
                    echo "Trying minimal configuration with -fPIC..."
                    ./configure --without-http_rewrite_module --with-cc-opt="-fPIC" >/tmp/nginx-configure.log 2>&1 || {
                        echo "Minimal configure also failed"
                        if [ -f /tmp/nginx-configure.log ]; then
                            tail -50 /tmp/nginx-configure.log || true
                        fi
                        exit 1
                    }
                })
                
                # Verify configure actually succeeded by checking for key files
                if [ ! -f "/tmp/nginx-$NGINX_VERSION/objs/ngx_modules.c" ]; then
                    echo "ERROR: Configure appeared to succeed but ngx_modules.c not found"
                    if [ -f /tmp/nginx-configure.log ]; then
                        echo "Full configure log:"
                        cat /tmp/nginx-configure.log
                    fi
                    exit 1
                fi
                
                # Show what was actually configured
                echo "Configure completed. Checking configured options..."
                if [ -f "/tmp/nginx-$NGINX_VERSION/objs/ngx_auto_config.h" ]; then
                    echo "Key configuration values:"
                    grep -E "NGINX_VER|NGX_PTR_SIZE|NGX_SIG_ATOMIC_T_SIZE|NGX_TIME_T_SIZE" "/tmp/nginx-$NGINX_VERSION/objs/ngx_auto_config.h" 2>/dev/null | head -10 || true
                    
                    # Verify nginx source is properly configured
                    # Note: build.rs will extract the signature during cargo build
                    # We just verify that the required files exist
                    if [ -f "/tmp/nginx-$NGINX_VERSION/objs/ngx_modules.c" ]; then
                        echo ""
                        echo "✓ Nginx source configured successfully"
                        echo "  build.rs will extract module signature from:"
                        echo "    - /tmp/nginx-$NGINX_VERSION/objs/ngx_modules.c (preferred)"
                        echo "    - /tmp/nginx-$NGINX_VERSION/objs/ngx_auto_config.h (fallback)"
                    elif [ -f "/tmp/nginx-$NGINX_VERSION/objs/ngx_auto_config.h" ]; then
                        echo ""
                        echo "⚠ Nginx source configured but ngx_modules.c not found"
                        echo "  build.rs will build signature from ngx_auto_config.h"
                        echo "  This may result in signature mismatch if feature detection logic differs"
                    fi
                fi
            else
                # Fallback to minimal configuration
                echo "No system configure args found, using minimal configuration..."
                (cd "/tmp/nginx-$NGINX_VERSION" && ./configure --without-http_rewrite_module --with-cc-opt="-fPIC" >/tmp/nginx-configure.log 2>&1 || {
                    echo "Minimal configure failed"
                    if [ -f /tmp/nginx-configure.log ]; then
                        tail -30 /tmp/nginx-configure.log || true
                    fi
                    exit 1
                })
            fi
            
            if [ -d "/tmp/nginx-$NGINX_VERSION/objs" ]; then
                NGINX_SOURCE_DIR="/tmp/nginx-$NGINX_VERSION"
                echo "Nginx source configured successfully"
            else
                echo "WARNING: Nginx configure may have failed. Check /tmp/nginx-configure.log"
                if [ -f /tmp/nginx-configure.log ]; then
                    echo "Last 30 lines of configure log:"
                    tail -30 /tmp/nginx-configure.log || true
                fi
            fi
        fi
    else
        echo "ERROR: Failed to download nginx source. Neither wget nor curl is available."
        exit 1
    fi
fi

if [ -z "$NGINX_SOURCE_DIR" ] || [ ! -f "$NGINX_SOURCE_DIR/objs/ngx_modules.c" ]; then
    echo "ERROR: Failed to find or configure nginx source for version $NGINX_VERSION"
    echo "Required file: $NGINX_SOURCE_DIR/objs/ngx_modules.c"
    exit 1
fi

echo "Using nginx source: $NGINX_SOURCE_DIR"

# Verify nginx source version matches system version
if [ -f "$NGINX_SOURCE_DIR/src/core/nginx.h" ]; then
    NGINX_SOURCE_VERSION=$(grep -E 'NGINX_VERSION' "$NGINX_SOURCE_DIR/src/core/nginx.h" | head -1 | sed -n 's/.*"\(.*\)".*/\1/p' || echo "")
    if [ -n "$NGINX_SOURCE_VERSION" ]; then
        echo "Nginx source version: $NGINX_SOURCE_VERSION"
        if [ "$NGINX_SOURCE_VERSION" != "$NGINX_VERSION" ]; then
            echo "WARNING: Nginx source version ($NGINX_SOURCE_VERSION) does not match system version ($NGINX_VERSION)"
            echo "This may cause binary compatibility issues"
        fi
    fi
fi

# Extract compiler flags from system nginx configure arguments
# This ensures binary compatibility by using the same compilation options
RUSTFLAGS_EXTRA=""
EXTRACTION_FAILED=0

if command -v nginx >/dev/null 2>&1; then
    NGINX_V_OUTPUT=$(nginx -V 2>&1)
    
    # Extract --with-cc-opt (compiler options)
    # Use sed to extract the value between single quotes
    CC_OPT=$(echo "$NGINX_V_OUTPUT" | sed -n "s/.*--with-cc-opt='\([^']*\)'.*/\1/p" || echo "")
    if [ -z "$CC_OPT" ]; then
        echo "WARNING: Failed to extract --with-cc-opt from nginx -V output"
        echo "This may cause binary compatibility issues"
        echo "nginx -V output: $NGINX_V_OUTPUT"
        EXTRACTION_FAILED=1
    else
        echo "Extracted --with-cc-opt: $CC_OPT"
        # Convert C compiler options to Rust linker arguments
        # Key options that affect binary compatibility:
        # -flto=auto -> -C link-arg=-flto=auto
        # -fPIC -> -C link-arg=-fPIC (already handled by Rust, but keep for consistency)
        # -fstack-protector-strong -> -C link-arg=-fstack-protector-strong
        for opt in $CC_OPT; do
            # Skip options that are not relevant for linking or might cause issues
            case "$opt" in
                -ffile-prefix-map=*|-fdebug-prefix-map=*|-Wdate-time|-D_*)
                    # Skip debug/map options that don't affect binary compatibility
                    ;;
                -flto=*|-fPIC|-fstack-protector*|-fcf-protection|-mno-omit-leaf-frame-pointer)
                    # These affect binary compatibility, add to RUSTFLAGS
                    RUSTFLAGS_EXTRA="$RUSTFLAGS_EXTRA -C link-arg=$opt"
                    ;;
                *)
                    # Other options might be important, add them
                    RUSTFLAGS_EXTRA="$RUSTFLAGS_EXTRA -C link-arg=$opt"
                    ;;
            esac
        done
    fi
    
    # Extract --with-ld-opt (linker options)
    # Use sed to extract the value between single quotes
    LD_OPT=$(echo "$NGINX_V_OUTPUT" | sed -n "s/.*--with-ld-opt='\([^']*\)'.*/\1/p" || echo "")
    if [ -z "$LD_OPT" ]; then
        echo "WARNING: Failed to extract --with-ld-opt from nginx -V output"
        echo "This may cause binary compatibility issues"
        EXTRACTION_FAILED=1
    else
        echo "Extracted --with-ld-opt: $LD_OPT"
        # Convert linker options to Rust linker arguments
        # -Wl,options -> -C link-arg=-Wl,options
        # Direct options -> -C link-arg=option
        for opt in $LD_OPT; do
            # These are critical for binary compatibility
            case "$opt" in
                -Wl,*)
                    # Already has -Wl, prefix, use as-is
                    RUSTFLAGS_EXTRA="$RUSTFLAGS_EXTRA -C link-arg=$opt"
                    ;;
                -flto=*|-ffat-lto-objects|-fPIC)
                    # LTO and PIC options
                    RUSTFLAGS_EXTRA="$RUSTFLAGS_EXTRA -C link-arg=$opt"
                    ;;
                *)
                    # Other linker options
                    RUSTFLAGS_EXTRA="$RUSTFLAGS_EXTRA -C link-arg=$opt"
                    ;;
            esac
        done
    fi
    
    if [ -n "$RUSTFLAGS_EXTRA" ]; then
        echo "Setting RUSTFLAGS for binary compatibility: $RUSTFLAGS_EXTRA"
        export RUSTFLAGS="$RUSTFLAGS_EXTRA"
    else
        if [ "$EXTRACTION_FAILED" -eq 1 ]; then
            echo "ERROR: Failed to extract compiler/linker options from system nginx"
            echo "Cannot ensure binary compatibility without matching compilation flags"
            echo "Please check nginx -V output format or report this issue"
            exit 1
        else
            echo "No compiler/linker options found in nginx configuration (this is unusual)"
            echo "Continuing with default RUSTFLAGS, but binary compatibility is not guaranteed"
        fi
    fi
else
    echo "ERROR: nginx command not found, cannot extract compilation flags"
    echo "Binary compatibility cannot be ensured"
    exit 1
fi

# Set up build environment
export NGINX_SOURCE_DIR="$NGINX_SOURCE_DIR"
export NGINX_BINARY_PATH="/usr/sbin/nginx"
export CARGO_FEATURES="--no-default-features"
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"

# Verify NGINX_SOURCE_DIR is set correctly
echo "Build environment:"
echo "  NGINX_SOURCE_DIR=$NGINX_SOURCE_DIR"
echo "  NGINX_BINARY_PATH=$NGINX_BINARY_PATH"
echo "  CARGO_FEATURES=$CARGO_FEATURES"
echo "  NGX_CONFIGURE_ARGS=$NGX_CONFIGURE_ARGS"
echo ""
echo "Note: build.rs will extract signature from system nginx binary and compare"
echo "      with signature from source config. If they don't match, a warning"
echo "      will be displayed but the system binary signature will be used."
if [ -n "$RUSTFLAGS" ]; then
    echo "  RUSTFLAGS=$RUSTFLAGS"
fi

# Verify nginx source has been configured correctly
# build.rs needs at least one of these files to extract module signature
if [ ! -f "$NGINX_SOURCE_DIR/objs/ngx_modules.c" ] && [ ! -f "$NGINX_SOURCE_DIR/objs/ngx_auto_config.h" ]; then
    echo "ERROR: Nginx source appears to be not configured properly"
    echo "Missing required files:"
    echo "  - $NGINX_SOURCE_DIR/objs/ngx_modules.c"
    echo "  - $NGINX_SOURCE_DIR/objs/ngx_auto_config.h"
    echo "At least one of these files is required for module signature extraction"
    exit 1
fi

# Check nginx version and key configuration values in objs/ngx_auto_config.h
if [ -f "$NGINX_SOURCE_DIR/objs/ngx_auto_config.h" ]; then
    echo "Nginx auto config found, checking version compatibility..."
    NGINX_BUILD_VERSION=$(grep -E 'NGINX_VER' "$NGINX_SOURCE_DIR/objs/ngx_auto_config.h" 2>/dev/null | head -1 | sed -n 's/.*"\(.*\)".*/\1/p' || echo "")
    if [ -n "$NGINX_BUILD_VERSION" ]; then
        echo "Nginx build version: $NGINX_BUILD_VERSION"
        if [ "$NGINX_BUILD_VERSION" != "$NGINX_VERSION" ]; then
            echo "WARNING: Build version ($NGINX_BUILD_VERSION) != System version ($NGINX_VERSION)"
        fi
    fi
    
    # Check key configuration values that affect binary compatibility
    echo "Checking key configuration values for binary compatibility:"
    grep -E "NGX_PTR_SIZE|NGX_SIG_ATOMIC_T_SIZE|NGX_TIME_T_SIZE|NGX_HAVE_DEBUG_MALLOC" "$NGINX_SOURCE_DIR/objs/ngx_auto_config.h" 2>/dev/null | head -10 || true
    
    # Compare with system nginx if possible
    if command -v nginx >/dev/null 2>&1; then
        echo "System nginx configuration (from nginx -V):"
        nginx -V 2>&1 | grep -E "configure arguments" || true
    fi
fi

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

# Build with verbose output to see what's happening
echo "Building module with cargo..."
echo "Command: cargo build --release $CARGO_FEATURES"
echo "NGINX_SOURCE_DIR=$NGINX_SOURCE_DIR"

cargo build --release $CARGO_FEATURES 2>&1 | tee /tmp/cargo-build.log || {
    echo "ERROR: Failed to build module"
    echo "Build logs:"
    tail -100 /tmp/cargo-build.log
    rm -rf "$BUILD_DIR"
    exit 1
}

echo "Build completed successfully"

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

# Verify module signature matches system nginx signature
echo "Verifying module signature compatibility..."
if command -v strings >/dev/null 2>&1 && [ -f "/usr/sbin/nginx" ]; then
    MODULE_SIG=$(strings "$MODULE_DIR/libnginx_x402.so" 2>/dev/null | grep -E '^[0-9]+,[0-9]+,[0-9]+,' | head -1 || echo "")
    NGINX_SIG=$(strings /usr/sbin/nginx 2>/dev/null | grep -E '^[0-9]+,[0-9]+,[0-9]+,' | head -1 || echo "")
    
    if [ -n "$MODULE_SIG" ] && [ -n "$NGINX_SIG" ]; then
        echo "Module signature: $MODULE_SIG"
        echo "Nginx signature:  $NGINX_SIG"
        if [ "$MODULE_SIG" = "$NGINX_SIG" ]; then
            echo "✓ Signatures match - module is binary compatible"
        else
            echo "ERROR: Signature mismatch - module may not be binary compatible"
            echo "This usually means:"
            echo "  1. Nginx source version used for build doesn't match system nginx version"
            echo "  2. Nginx source configure arguments don't match system nginx configuration"
            echo "  3. Module signature extraction failed or was incorrect"
            echo ""
            echo "Please check:"
            echo "  - System nginx version: $(nginx -v 2>&1 | grep -oE 'nginx/[0-9.]+' || echo 'unknown')"
            echo "  - Build nginx source version: ${NGINX_SOURCE_VERSION:-unknown}"
            echo "  - Build logs: /tmp/cargo-build.log"
            rm -rf "$BUILD_DIR"
            exit 1
        fi
    else
        echo "WARNING: Could not extract signatures for verification"
        echo "Module signature: ${MODULE_SIG:-not found}"
        echo "Nginx signature:  ${NGINX_SIG:-not found}"
    fi
else
    echo "WARNING: Cannot verify signatures (strings command or nginx binary not found)"
fi

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
* Mon Nov 27 2025 Ryan Kung <ryan@polyjuice.io> - 1.0.4-1
- Version bump to 1.0.4
- Fixed workflow build errors by adding nginx source setup steps
- Added Makefile for easier cargo publish workflow
* Mon Nov 27 2025 Ryan Kung <ryan@polyjuice.io> - 1.0.3-1
- Version bump to 1.0.3
- Improved build.rs to support NGX_VERSION environment variable
- Enhanced nginx version detection with fallback support

* Mon Nov 27 2025 Ryan Kung <ryan@polyjuice.io> - 1.0.2-1
- Version bump to 1.0.2
- Simplified README documentation

* Mon Nov 24 2025 Ryan Kung <ryan@polyjuice.io> - 1.0.1-1
- Implement custom auto-detect nginx version functionality
- Automatically downloads matching nginx source during build
- Removed vendored feature dependency
- Ensures module version matches system nginx version
