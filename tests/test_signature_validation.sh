#!/bin/bash
# Test script to validate nginx module signatures by loading the module
# This script builds modules using Dockerfile.debian.test and Dockerfile.centos.test,
# extracts signatures, compares them, and verifies module loading

set -e

echo "=========================================="
echo "Validating nginx module signatures by loading x402 module"
echo "=========================================="

# Function to test a distribution
test_distribution() {
    local distro=$1
    local dockerfile=$2
    local module_path=$3
    
    echo ""
    echo "=========================================="
    echo "Testing $distro"
    echo "=========================================="
    
    IMAGE_NAME="nginx-x402-signature-validation-$distro"
    
    # Build Docker image (this builds the module)
    echo "Building Docker image with module..."
    docker build --no-cache -t "$IMAGE_NAME" -f "$dockerfile" . || {
        echo "ERROR: Docker build failed for $distro"
        return 1
    }
    
    # Extract and compare signatures
    echo ""
    echo "=== Extracting signatures ==="
    SIGNATURE_INFO=$(docker run --rm "$IMAGE_NAME" bash -c "
        # Extract signature from module
        MODULE_SIG=\$(strings \"$module_path\" 2>/dev/null | grep -E '^[0-9]+,[0-9]+,[0-9]+,[01]+$' | head -1)
        # Extract signature from system nginx binary
        NGINX_SIG=\$(strings /usr/sbin/nginx 2>/dev/null | grep -E '^[0-9]+,[0-9]+,[0-9]+,[01]+$' | head -1)
        
        if [ -z \"\$MODULE_SIG\" ]; then
            echo 'ERROR: Could not extract signature from module'
            exit 1
        fi
        if [ -z \"\$NGINX_SIG\" ]; then
            echo 'ERROR: Could not extract signature from system nginx binary'
            exit 1
        fi
        
        echo \"MODULE_SIG|\$MODULE_SIG\"
        echo \"NGINX_SIG|\$NGINX_SIG\"
        
        if [ \"\$MODULE_SIG\" = \"\$NGINX_SIG\" ]; then
            echo 'MATCH|yes'
        else
            echo 'MATCH|no'
            echo \"MISMATCH_DETAIL|Module: \$MODULE_SIG vs Nginx: \$NGINX_SIG\"
        fi
    ")
    
    # Parse signature info
    MODULE_SIG=$(echo "$SIGNATURE_INFO" | grep "^MODULE_SIG|" | cut -d'|' -f2)
    NGINX_SIG=$(echo "$SIGNATURE_INFO" | grep "^NGINX_SIG|" | cut -d'|' -f2)
    MATCH=$(echo "$SIGNATURE_INFO" | grep "^MATCH|" | cut -d'|' -f2)
    
    echo "Module signature: $MODULE_SIG"
    echo "Nginx signature:  $NGINX_SIG"
    
    if [ "$MATCH" != "yes" ]; then
        echo "ERROR: Signature mismatch!"
        echo "$SIGNATURE_INFO" | grep "MISMATCH_DETAIL"
        return 1
    fi
    
    echo "✓ Signatures match!"
    
    # Test nginx configuration (this will verify signature compatibility)
    echo ""
    echo "=== Testing nginx configuration (signature validation) ==="
    CONFIG_TEST=$(docker run --rm "$IMAGE_NAME" nginx -t 2>&1) || {
        echo "ERROR: nginx configuration test failed"
        echo "$CONFIG_TEST"
        if echo "$CONFIG_TEST" | grep -q "binary compatible"; then
            echo ""
            echo "This indicates a signature mismatch - nginx cannot load the module"
        fi
        return 1
    }
    
    echo "$CONFIG_TEST"
    echo "✓ Nginx configuration test passed (signature validated)"
    
    # Test module loading by starting nginx
    echo ""
    echo "=== Testing module loading ==="
    CONTAINER_ID=$(docker run -d --name "nginx-test-$distro" "$IMAGE_NAME" nginx -g 'daemon off;')
    sleep 5
    
    # Check if nginx started successfully by checking HTTP response
    # This is more reliable than checking processes (pgrep/ps may not be available)
    if docker exec "nginx-test-$distro" curl -f -s http://localhost/health > /dev/null 2>&1; then
        echo "✓ Nginx started successfully (module loaded)"
    else
        echo "ERROR: Nginx failed to start or respond"
        echo "Container logs:"
        docker logs "nginx-test-$distro" 2>&1 | tail -20
        docker rm -f "nginx-test-$distro" > /dev/null 2>&1
        return 1
    fi
    
    # Test module functionality
    echo ""
    echo "=== Testing module functionality ==="
    if docker exec "nginx-test-$distro" curl -f http://localhost/health > /dev/null 2>&1; then
        echo "✓ Module health check passed"
    else
        echo "ERROR: Module health check failed"
        docker logs "nginx-test-$distro"
        docker rm -f "nginx-test-$distro" > /dev/null 2>&1
        return 1
    fi
    
    # Check nginx error log for signature-related errors
    ERROR_LOG=$(docker exec "nginx-test-$distro" cat /var/log/nginx/error.log 2>/dev/null || echo "")
    if echo "$ERROR_LOG" | grep -qi "binary compatible\|signature"; then
        echo "WARNING: Found signature-related errors in nginx error log:"
        echo "$ERROR_LOG" | grep -i "binary compatible\|signature"
        docker rm -f "nginx-test-$distro" > /dev/null 2>&1
        return 1
    fi
    
    # Cleanup
    docker rm -f "nginx-test-$distro" > /dev/null 2>&1
    
    echo ""
    echo "✓ $distro test passed - signature is valid and module loads correctly"
    return 0
}

# Test Debian
if [ -f "tests/Dockerfile.debian.test" ]; then
    test_distribution "debian" "tests/Dockerfile.debian.test" "/usr/lib/nginx/modules/libnginx_x402.so"
    DEBIAN_RESULT=$?
else
    echo "WARNING: tests/Dockerfile.debian.test not found"
    DEBIAN_RESULT=1
fi

# Test CentOS
if [ -f "tests/Dockerfile.centos.test" ]; then
    test_distribution "centos" "tests/Dockerfile.centos.test" "/usr/lib64/nginx/modules/libnginx_x402.so"
    CENTOS_RESULT=$?
else
    echo "WARNING: tests/Dockerfile.centos.test not found"
    CENTOS_RESULT=1
fi

echo ""
echo "=========================================="
if [ $DEBIAN_RESULT -eq 0 ] && [ $CENTOS_RESULT -eq 0 ]; then
    echo "✓ All signature validation tests passed"
    echo "✓ Debian signature is valid and module loads correctly"
    echo "✓ CentOS signature is valid and module loads correctly"
    exit 0
else
    echo "✗ Some signature validation tests failed"
    [ $DEBIAN_RESULT -ne 0 ] && echo "✗ Debian test failed"
    [ $CENTOS_RESULT -ne 0 ] && echo "✗ CentOS test failed"
    exit 1
fi

