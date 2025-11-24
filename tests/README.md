# Integration Test Documentation

## Overview

This project includes two types of integration tests:

1. **Basic Integration Tests** (`integration_test.rs`) - Requires manual nginx setup
2. **Docker Integration Tests** (`docker_integration_test.rs`) - Automated testing with Docker

## Quick Start

### Method 1: Using Automated Script (Recommended)

```bash
# Run complete integration test workflow
./tests/run_integration_tests.sh
```

This script will:
1. Build the module
2. Build Docker test image
3. Start test container
4. Run tests
5. Clean up resources

### Method 2: Manual Execution

#### Step 1: Build Module

```bash
export NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
cargo build --release
```

#### Step 2: Build Docker Image

```bash
docker build -t nginx-x402-test -f tests/Dockerfile.test .
```

#### Step 3: Start Test Container

```bash
docker run -d --name nginx-x402-test-container -p 8080:80 nginx-x402-test
```

#### Step 4: Run Tests

```bash
# Wait for nginx to start
sleep 5

# Run integration tests
cargo test --test integration_test -- --ignored

# Or run Docker integration tests
cargo test --test docker_integration_test --features integration-test -- --ignored
```

#### Step 5: Cleanup

```bash
docker stop nginx-x402-test-container
docker rm nginx-x402-test-container
```

## Test Descriptions

### Basic Integration Tests (`integration_test.rs`)

These tests require:
- Built module file
- Running nginx instance (with module loaded)
- Nginx configured with test endpoints

**Run:**
```bash
cargo test --test integration_test -- --ignored
```

**Test Coverage:**
- ✅ Module loading test
- ✅ 402 response test (without payment header)
- ✅ Public endpoint access test
- ✅ Metrics endpoint test

### Docker Integration Tests (`docker_integration_test.rs`)

These tests use Docker to automate the entire process:
- Automatically build Docker image
- Automatically start container
- Automatically run tests
- Automatically clean up

**Run:**
```bash
cargo test --test docker_integration_test --features integration-test -- --ignored
```

**Requirements:**
- Docker installed and running
- Port 8080 available

## CI/CD Integration

Integration tests are configured in GitHub Actions but skipped by default (using `--ignored` flag).

To enable integration tests in CI:

1. Ensure CI environment has Docker support
2. Remove `#[ignore]` attribute from tests
3. Or explicitly run with `cargo test -- --ignored`

## Test Configuration

### Nginx Test Configuration

Test nginx configuration is located at `tests/nginx.test.conf`, containing:
- Module loading configuration
- Test endpoint configuration
- Health check endpoint
- Protected endpoint (requires payment)
- Metrics endpoint

### Docker Test Image

`tests/Dockerfile.test` creates an image containing:
- Ubuntu base image
- Nginx
- Built module
- Test configuration

## Troubleshooting

### Docker Not Running

```bash
# Check Docker status
docker --version
docker ps

# Start Docker (Linux)
sudo systemctl start docker
```

### Port Already in Use

If port 8080 is occupied, you can modify:
- `NGINX_PORT` constant in `docker_integration_test.rs`
- Port mapping in Docker run command

### Module Not Found

Ensure module is built:
```bash
ls -la target/release/libnginx_x402.so
# Or
find target -name "libnginx_x402.so"
```

### Nginx Startup Failed

View container logs:
```bash
docker logs nginx-x402-test-container
```

Check nginx configuration:
```bash
docker exec nginx-x402-test-container nginx -t
```

## Extending Tests

### Adding New Tests

1. Add test function in `integration_test.rs` or `docker_integration_test.rs`
2. Use `#[test]` and `#[ignore]` attributes
3. Use provided helper functions (`http_request`, `http_get`, `wait_for_nginx`, etc.)

### Testing Different Configurations

You can create multiple nginx configuration files:
- `nginx.test.conf` - Basic test configuration
- `nginx.test.prod.conf` - Production environment test
- `nginx.test.perf.conf` - Performance test configuration

Then specify different config files in Dockerfile.

## Performance Testing

You can use the following tools for performance testing:

```bash
# Using wrk
wrk -t12 -c400 -d30s http://localhost:8080/api/protected

# Using ab
ab -n 10000 -c 100 http://localhost:8080/api/protected

# Using curl for simple test
curl -v http://localhost:8080/api/protected
```

## Related Files

- `tests/integration_test.rs` - Basic integration tests
- `tests/docker_integration_test.rs` - Docker integration tests
- `tests/Dockerfile.test` - Docker test image
- `tests/nginx.test.conf` - Nginx test configuration
- `tests/run_integration_tests.sh` - Automated test script
