# RPM Package Build Instructions

This directory contains the RPM package specification file for building nginx-x402 RPM packages.

## Prerequisites

- `rpmbuild` tool (usually provided by `rpm-build` package)
- Rust toolchain (cargo, rustc)
- Build dependencies: openssl-devel, pkgconfig, clang-devel

## Building RPM Package

### Method 1: Using rpmbuild directly

```bash
# Install build dependencies (Fedora/RHEL/CentOS)
sudo dnf install rpm-build cargo rustc openssl-devel pkgconfig clang-devel

# Create source tarball
cd /path/to/nginx-x402
tar -czf ~/rpmbuild/SOURCES/nginx-x402-0.1.4.tar.gz \
    --exclude='.git' \
    --exclude='target' \
    --exclude='*.deb' \
    --exclude='*.rpm' \
    --transform 's,^,nginx-x402-0.1.4/,' \
    .

# Build RPM
rpmbuild -ba rpm/nginx-x402.spec
```

### Method 2: Using mock (recommended for clean builds)

```bash
# Install mock
sudo dnf install mock

# Add user to mock group
sudo usermod -a -G mock $USER

# Create source tarball
cd /path/to/nginx-x402
tar -czf ~/rpmbuild/SOURCES/nginx-x402-0.1.4.tar.gz \
    --exclude='.git' \
    --exclude='target' \
    --exclude='*.deb' \
    --exclude='*.rpm' \
    --transform 's,^,nginx-x402-0.1.4/,' \
    .

# Build RPM in mock chroot
mock -r fedora-rawhide-x86_64 --buildsrpm --spec rpm/nginx-x402.spec --sources ~/rpmbuild/SOURCES/
mock -r fedora-rawhide-x86_64 --rebuild ~/rpmbuild/SRPMS/nginx-x402-0.1.4-1.fc*.src.rpm
```

## Architecture Support

The RPM spec file supports multiple architectures:
- x86_64 (amd64)
- aarch64 (arm64)
- armv7hl (armhf)

For cross-compilation, set the `RUST_TARGET` environment variable:

```bash
# For aarch64
export RUST_TARGET=aarch64-unknown-linux-gnu
rpmbuild -ba rpm/nginx-x402.spec --target aarch64

# For armv7hl
export RUST_TARGET=armv7-unknown-linux-gnueabihf
rpmbuild -ba rpm/nginx-x402.spec --target armv7hl
```

## Installation

After building, install the RPM package:

```bash
sudo rpm -ivh ~/rpmbuild/RPMS/x86_64/nginx-x402-0.1.4-1.fc*.x86_64.rpm
# Or
sudo dnf install ~/rpmbuild/RPMS/x86_64/nginx-x402-0.1.4-1.fc*.x86_64.rpm
```

## Module Configuration

After installation, enable the module in Nginx using **ONE** of these methods:

**Option 1: Using modules-enabled (Recommended)**
```bash
sudo cp /etc/nginx/modules-available/x402.conf /etc/nginx/modules-enabled/
```

**Option 2: Manual load_module in nginx.conf**
```bash
echo "load_module /usr/lib64/nginx/modules/libnginx_x402.so;" | sudo tee -a /etc/nginx/nginx.conf
```

**⚠️ IMPORTANT: Use ONLY ONE method. Using both will cause "module already loaded" errors.**

## Environment Variables

The build process uses the following environment variables:

- `NGX_CONFIGURE_ARGS`: Nginx configure arguments (default: `--without-http_rewrite_module`)
- `LIBCLANG_PATH`: Path to libclang library (auto-detected if not set)
- `RUST_TARGET`: Rust target triple for cross-compilation (optional)

## Troubleshooting

### Build fails with "PCRE library required"

Set `NGX_CONFIGURE_ARGS` to disable rewrite module:
```bash
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
```

### Build fails with "libclang not found"

Install clang-devel and set LIBCLANG_PATH:
```bash
sudo dnf install clang-devel
export LIBCLANG_PATH=/usr/lib64/llvm/lib
```

### Module not found after installation

Check the module path:
- Fedora/RHEL: `/usr/lib64/nginx/modules/libnginx_x402.so`
- Some distributions: `/usr/lib/nginx/modules/libnginx_x402.so`

Update the `load_module` directive in nginx.conf accordingly.

