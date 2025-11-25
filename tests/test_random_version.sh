#!/bin/bash
# Random nginx version integration test
# Tests the module with a randomly selected nginx version

set -e

# Available nginx versions to test
VERSIONS=(
    "1.24.0"
    "1.25.0"
    "1.26.0"
    "1.27.0"
    "1.28.0"
)

# If a version is provided as argument, use it; otherwise pick random
if [ -n "$1" ]; then
    NGINX_VERSION="$1"
    echo "Testing with specified version: $NGINX_VERSION"
else
    # Pick a random version
    RANDOM_INDEX=$((RANDOM % ${#VERSIONS[@]}))
    NGINX_VERSION="${VERSIONS[$RANDOM_INDEX]}"
    echo "Testing with random version: $NGINX_VERSION"
fi

echo "=========================================="
echo "Testing nginx-x402 with nginx $NGINX_VERSION"
echo "=========================================="

# Create a temporary Dockerfile with the selected version
TEMP_DOCKERFILE="tests/Dockerfile.test.$NGINX_VERSION"
sed "s/nginx-1.28.0/nginx-$NGINX_VERSION/g; s|/tmp/nginx-1\.28\.0|/tmp/nginx-$NGINX_VERSION|g" tests/Dockerfile.test > "$TEMP_DOCKERFILE"

# Build the Docker image
IMAGE_NAME="nginx-x402-test-$NGINX_VERSION"
echo "Building Docker image: $IMAGE_NAME"
docker build --no-cache -t "$IMAGE_NAME" -f "$TEMP_DOCKERFILE" . || {
    echo "ERROR: Docker build failed for nginx $NGINX_VERSION"
    rm -f "$TEMP_DOCKERFILE"
    exit 1
}

# Test nginx configuration
echo "Testing nginx configuration..."
docker run --rm "$IMAGE_NAME" nginx -t || {
    echo "ERROR: nginx configuration test failed for nginx $NGINX_VERSION"
    rm -f "$TEMP_DOCKERFILE"
    exit 1
}

# Verify signature match
echo "Verifying module signature..."
SIGNATURE_MATCH=$(docker run --rm "$IMAGE_NAME" sh -c "
    MODULE_SIG=\$(strings /usr/lib/nginx/modules/libnginx_x402.so | grep -E '^[0-9]+,[0-9]+,[0-9]+,' | head -1)
    NGINX_SIG=\$(strings /usr/sbin/nginx | grep -E '^[0-9]+,[0-9]+,[0-9]+,' | head -1)
    if [ \"\$MODULE_SIG\" = \"\$NGINX_SIG\" ]; then
        echo 'MATCH'
    else
        echo 'MISMATCH'
        echo \"Module: \$MODULE_SIG\"
        echo \"Nginx:  \$NGINX_SIG\"
    fi
")

if [ "$SIGNATURE_MATCH" = "MATCH" ]; then
    echo "✓ Signatures match!"
else
    echo "ERROR: Signature mismatch for nginx $NGINX_VERSION"
    echo "$SIGNATURE_MATCH"
    rm -f "$TEMP_DOCKERFILE"
    exit 1
fi

# Test module loading and basic functionality
echo "Testing module functionality..."
CONTAINER_ID=$(docker run -d --name "nginx-test-$NGINX_VERSION" "$IMAGE_NAME")
sleep 3

if docker exec "nginx-test-$NGINX_VERSION" curl -f http://localhost/health > /dev/null 2>&1; then
    echo "✓ Module loaded and responding correctly"
else
    echo "ERROR: Module health check failed for nginx $NGINX_VERSION"
    docker logs "nginx-test-$NGINX_VERSION"
    docker rm -f "nginx-test-$NGINX_VERSION" > /dev/null 2>&1
    rm -f "$TEMP_DOCKERFILE"
    exit 1
fi

docker rm -f "nginx-test-$NGINX_VERSION" > /dev/null 2>&1

# Cleanup
rm -f "$TEMP_DOCKERFILE"

echo "=========================================="
echo "✓ All tests passed for nginx $NGINX_VERSION"
echo "=========================================="

