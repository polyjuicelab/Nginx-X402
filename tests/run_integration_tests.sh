#!/bin/bash
# Script to run integration tests
# This script builds the module, creates a Docker image, and runs tests

set -e

echo "=== Nginx-x402 Integration Test Runner ==="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed or not in PATH${NC}"
    exit 1
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed or not in PATH${NC}"
    exit 1
fi

# Check if Docker daemon is running
if ! docker ps > /dev/null 2>&1; then
    echo -e "${RED}Error: Docker daemon is not running. Please start Docker and try again.${NC}"
    exit 1
fi

echo -e "${YELLOW}Step 1: Building Docker test image (module will be built inside container)...${NC}"
if ! docker build -t nginx-x402-test -f tests/Dockerfile.test .; then
    echo -e "${RED}Failed to build Docker image${NC}"
    exit 1
fi

echo -e "${YELLOW}Step 2: Cleaning up any existing containers...${NC}"
docker stop nginx-x402-test-container 2>/dev/null || true
docker rm nginx-x402-test-container 2>/dev/null || true

echo -e "${YELLOW}Step 3: Starting test container...${NC}"
CONTAINER_ID=$(docker run -d --name nginx-x402-test-container -p 8080:80 nginx-x402-test)
echo -e "${GREEN}Container started: $CONTAINER_ID${NC}"

# Wait for nginx to be ready
echo -e "${YELLOW}Step 4: Waiting for nginx to be ready...${NC}"
for i in {1..30}; do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${GREEN}Nginx is ready!${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}Nginx did not become ready in time${NC}"
        docker logs nginx-x402-test-container
        docker stop nginx-x402-test-container
        docker rm nginx-x402-test-container
        exit 1
    fi
    sleep 1
done

echo -e "${YELLOW}Step 5: Running integration tests...${NC}"
if cargo test --test integration_test -- --nocapture; then
    echo -e "${GREEN}All integration tests passed!${NC}"
    TEST_RESULT=0
else
    echo -e "${RED}Some integration tests failed${NC}"
    TEST_RESULT=1
fi

echo -e "${YELLOW}Step 6: Cleaning up...${NC}"
docker stop nginx-x402-test-container
docker rm nginx-x402-test-container

if [ $TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}=== Integration tests completed successfully ===${NC}"
else
    echo -e "${RED}=== Integration tests failed ===${NC}"
fi

exit $TEST_RESULT

