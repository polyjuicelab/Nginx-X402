#!/bin/bash
# Collect nginx module signatures from supported distributions
# This script builds the module on each supported distribution and extracts signatures

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SIGNATURES_FILE="$PROJECT_ROOT/signatures.json"

echo "Collecting nginx module signatures from supported distributions..."
echo "Output file: $SIGNATURES_FILE"

# Initialize signatures file
cat > "$SIGNATURES_FILE" <<EOF
{
  "timestampgenerated": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "signatures": []
}
EOF

# Function to collect signature from a Docker image
collect_signature() {
    local distro=$1
    local version=$2
    local dockerfile=$3
    local image_name="nginx-x402-signature-collector-$distro-$version"
    
    echo ""
    echo "=========================================="
    echo "Collecting signature from $distro $version"
    echo "=========================================="
    
    # Build Docker image (but we only need nginx installed, not the module built)
    echo "Building Docker image..."
    docker build -q -t "$image_name" -f "$dockerfile" "$PROJECT_ROOT" || {
        echo "ERROR: Failed to build Docker image for $distro $version"
        return 1
    }
    
    # Extract signature directly from system nginx binary - NO NEED TO BUILD NGINX SOURCE
    echo "Extracting signature from system nginx..."
    docker run --rm "$image_name" bash -c "
        NGINX_VERSION=\$(nginx -v 2>&1 | sed -n 's/.*nginx\/\([0-9]\+\.[0-9]\+\.[0-9]\+\).*/\1/p' | head -1)
        NGINX_CONFIGURE_ARGS=\$(nginx -V 2>&1 | grep -oE 'configure arguments:.*' | sed 's/configure arguments://' || echo '')
        # Extract signature directly from system nginx binary
        # Signature format: ^[0-9]+,[0-9]+,[0-9]+,[01]+$
        # This format is unique - nginx stores module signature as a string in the binary
        # We use strict regex to ensure accuracy: must start with digits, have 3 commas, end with binary flags
        NGINX_SIG=\$(strings /usr/sbin/nginx 2>/dev/null | grep -E '^[0-9]+,[0-9]+,[0-9]+,[01]+$' | head -1)
        if [ -z \"\$NGINX_SIG\" ]; then
            # Try alternative path
            NGINX_SIG=\$(strings /usr/local/sbin/nginx 2>/dev/null | grep -E '^[0-9]+,[0-9]+,[0-9]+,[01]+$' | head -1)
        fi
        # Verify we found exactly one signature (should be unique)
        SIG_COUNT=\$(strings /usr/sbin/nginx 2>/dev/null | grep -E '^[0-9]+,[0-9]+,[0-9]+,[01]+$' | wc -l)
        if [ \"\$SIG_COUNT\" -gt 1 ]; then
            echo \"WARNING: Found \$SIG_COUNT signatures, using first one\" >&2
        fi
        CONFIGURE_HASH=\$(echo \"\$NGINX_CONFIGURE_ARGS\" | sha256sum | cut -d' ' -f1)
        COLLECTED_AT=\$(date -u +%Y-%m-%dT%H:%M:%SZ)
        echo \"NGINX_VERSION|\$NGINX_VERSION\"
        echo \"NGINX_CONFIGURE_ARGS|\$NGINX_CONFIGURE_ARGS\"
        echo \"NGINX_SIG|\$NGINX_SIG\"
        echo \"MODULE_SIG|\"
        echo \"CONFIGURE_HASH|\$CONFIGURE_HASH\"
        echo \"COLLECTED_AT|\$COLLECTED_AT\"
    " > /tmp/signature_data_${distro}_${version}.txt
    
    # Parse the extracted data using | as delimiter
    while IFS='|' read -r key value; do
        case "$key" in
            NGINX_VERSION) NGINX_VERSION="$value" ;;
            NGINX_CONFIGURE_ARGS) NGINX_CONFIGURE_ARGS="$value" ;;
            NGINX_SIG) NGINX_SIG="$value" ;;
            MODULE_SIG) MODULE_SIG="$value" ;;
            CONFIGURE_HASH) CONFIGURE_HASH="$value" ;;
            COLLECTED_AT) COLLECTED_AT="$value" ;;
        esac
    done < /tmp/signature_data_${distro}_${version}.txt
    
    # Escape configure args for JSON
    ESCAPED_ARGS=$(echo "$NGINX_CONFIGURE_ARGS" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g')
    
    # Create JSON entry
    if command -v jq >/dev/null 2>&1; then
        JSON_ENTRY=$(jq -n \
            --arg distro "$distro" \
            --arg version "$version" \
            --arg nginx_version "$NGINX_VERSION" \
            --arg configure_args "$ESCAPED_ARGS" \
            --arg configure_hash "$CONFIGURE_HASH" \
            --arg signature "$NGINX_SIG" \
            --arg module_sig "$MODULE_SIG" \
            --arg collected_at "$COLLECTED_AT" \
            '{distro: $distro, distroversion: $version, nginxversion: $nginx_version, configureargs: $configure_args, configurehash: $configure_hash, signature: $signature, modulesignature: $module_sig, collectedat: $collected_at}')
        
        # Add to signatures file
        jq ".signatures += [$JSON_ENTRY]" "$SIGNATURES_FILE" > "$SIGNATURES_FILE.tmp" && \
        mv "$SIGNATURES_FILE.tmp" "$SIGNATURES_FILE"
    else
        echo "WARNING: jq not found, saving raw data to /tmp/signature_${distro}_${version}.json"
        cat > /tmp/signature_${distro}_${version}.json <<EOF
{
  "distro": "$distro",
  "distroversion": "$version",
  "nginxversion": "$NGINX_VERSION",
  "configureargs": "$ESCAPED_ARGS",
  "configurehash": "$CONFIGURE_HASH",
  "signature": "$NGINX_SIG",
  "modulesignature": "$MODULE_SIG",
  "collectedat": "$COLLECTED_AT"
}
EOF
    fi
    
    echo "✓ Collected signature from $distro $version"
    echo "  Nginx version: $NGINX_VERSION"
    echo "  Signature: $NGINX_SIG"
}

# Collect signatures from supported distributions
echo "Collecting signatures..."

# Debian
if [ -f "$PROJECT_ROOT/tests/Dockerfile.debian.test" ]; then
    collect_signature "debian" "bookworm" "tests/Dockerfile.debian.test"
fi

# Ubuntu  
if [ -f "$PROJECT_ROOT/tests/Dockerfile.test" ]; then
    collect_signature "ubuntu" "22.04" "tests/Dockerfile.test"
fi

# CentOS (skip 8 as it's EOL, use 9 only)
if [ -f "$PROJECT_ROOT/tests/Dockerfile.centos.test" ]; then
    collect_signature "centos" "9" "tests/Dockerfile.centos.test" || echo "WARNING: Failed to collect CentOS 9 signature"
fi

echo ""
echo "=========================================="
echo "✓ Signature collection complete"
echo "Signatures saved to: $SIGNATURES_FILE"
if command -v jq >/dev/null 2>&1; then
    echo "Collected $(jq '.signatures | length' "$SIGNATURES_FILE") signatures"
fi
echo "=========================================="
