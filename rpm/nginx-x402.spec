%define modulename nginx-x402
%define moduledir %{_libdir}/nginx/modules

Name:           nginx-x402
Version:        0.1.4
Release:        1%{?dist}
Summary:        Pure Rust Nginx module for x402 HTTP micropayment protocol
License:        AGPL-3.0
URL:            https://github.com/polyjuicelab/Nginx-X402
Source0:        %{name}-%{version}.tar.gz
# BuildArch defaults to host architecture
# For cross-compilation, we'll handle architecture in the build process
BuildArch:      %{_arch}

BuildRequires:  cargo
BuildRequires:  rustc
BuildRequires:  openssl-devel
BuildRequires:  pkgconfig
BuildRequires:  clang-devel
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
# Set required environment variables for vendored feature
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
if [ -d /usr/lib64/llvm/lib ]; then
    export LIBCLANG_PATH=/usr/lib64/llvm/lib
elif [ -d /usr/lib/llvm/lib ]; then
    export LIBCLANG_PATH=/usr/lib/llvm/lib
fi

# Determine target architecture from RUST_TARGET or use default
# Map Rust target triple to RPM architecture name
if [ -n "$RUST_TARGET" ]; then
    case "$RUST_TARGET" in
        x86_64-unknown-linux-gnu)
            TARGET_ARCH="x86_64"
            ;;
        aarch64-unknown-linux-gnu)
            TARGET_ARCH="aarch64"
            # Set CC for target architecture only (not global CC)
            # ring crate's build script needs host CC, but C code needs target CC
            export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
            # Setup pkg-config for cross-compilation
            export PKG_CONFIG_ALLOW_CROSS=1
            export PKG_CONFIG_PATH_aarch64_unknown_linux_gnu=/usr/lib/aarch64-linux-gnu/pkgconfig
            if command -v aarch64-linux-gnu-pkg-config >/dev/null 2>&1; then
                export PKG_CONFIG_aarch64_unknown_linux_gnu=aarch64-linux-gnu-pkg-config
            fi
            ;;
        armv7-unknown-linux-gnueabihf)
            TARGET_ARCH="armv7hl"
            # Set CC for target architecture only (not global CC)
            # ring crate's build script needs host CC, but C code needs target CC
            export CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc
            # Setup pkg-config for cross-compilation
            export PKG_CONFIG_ALLOW_CROSS=1
            export PKG_CONFIG_PATH_armv7_unknown_linux_gnueabihf=/usr/lib/arm-linux-gnueabihf/pkgconfig
            if command -v arm-linux-gnueabihf-pkg-config >/dev/null 2>&1; then
                export PKG_CONFIG_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-pkg-config
            fi
            # For openssl-sys, we need to set OPENSSL_DIR for cross-compilation
            # Try to find OpenSSL in the cross-compilation sysroot
            if [ -d /usr/arm-linux-gnueabihf ]; then
                export OPENSSL_DIR=/usr/arm-linux-gnueabihf
            elif [ -d /usr/lib/arm-linux-gnueabihf ]; then
                export OPENSSL_DIR=/usr/lib/arm-linux-gnueabihf
            fi
            ;;
        *)
            TARGET_ARCH="x86_64"
            ;;
    esac
    # Override BuildArch for cross-compilation
    # Note: This is a workaround - rpmbuild doesn't support --target for cross-compilation
    # We build the binary first, then rpmbuild will package it
    rustup target add $RUST_TARGET || true
    cargo build --release --target $RUST_TARGET
else
    TARGET_ARCH="x86_64"
    cargo build --release
fi

%install
# Create directories
mkdir -p %{buildroot}%{moduledir}
mkdir -p %{buildroot}%{_docdir}/%{name}
mkdir -p %{buildroot}%{_sysconfdir}/nginx/modules-available

# Install module (try architecture-specific path first, then fallback)
if [ -n "$RUST_TARGET" ] && [ "$RUST_TARGET" != "x86_64-unknown-linux-gnu" ]; then
    if [ -f "target/$RUST_TARGET/release/libnginx_x402.so" ]; then
        cp target/$RUST_TARGET/release/libnginx_x402.so %{buildroot}%{moduledir}/libnginx_x402.so
    else
        find target/$RUST_TARGET -name "libnginx_x402.so" -type f | head -1 | xargs -I {} cp {} %{buildroot}%{moduledir}/libnginx_x402.so || true
    fi
else
    find target -name "libnginx_x402.so" -type f | head -1 | xargs -I {} cp {} %{buildroot}%{moduledir}/libnginx_x402.so || \
    cp target/release/libnginx_x402.so %{buildroot}%{moduledir}/libnginx_x402.so || true
fi

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
* Mon Jan 20 2025 Ryan Kung <ryan@polyjuice.io> - 0.1.4-1
- Initial RPM package release

