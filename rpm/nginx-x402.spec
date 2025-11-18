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

# Build for native architecture (x86_64)
cargo build --release

%install
# Create directories
mkdir -p %{buildroot}%{moduledir}
mkdir -p %{buildroot}%{_docdir}/%{name}
mkdir -p %{buildroot}%{_sysconfdir}/nginx/modules-available

# Install module (native build)
cp target/release/libnginx_x402.so %{buildroot}%{moduledir}/libnginx_x402.so

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

