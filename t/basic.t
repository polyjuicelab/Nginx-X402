#!/usr/bin/env perl

# Test::Nginx test suite for nginx-x402 module
#
# This test suite verifies the x402 ngx-rust module works correctly.
#
# Prerequisites:
# 1. Build the module: cargo build --release --features ngx-rust
# 2. Install Test::Nginx: cpan Test::Nginx
# 3. Set TEST_NGINX_BINARY to point to your nginx binary
# 4. Configure Nginx to load the module: load_module /path/to/libnginx_x402.so;

use Test::Nginx::Socket 'no_plan';

repeat_each(1);
log_level('info');

# Find library path
our $nginx_lib_dir = $ENV{'TEST_NGINX_LIB_DIR'} || 'target/aarch64-apple-darwin/release';
our $nginx_lib = "$nginx_lib_dir/libnginx_x402.dylib";

# Set library path for dynamic loading
if (-f $nginx_lib) {
    $ENV{'DYLD_LIBRARY_PATH'} = $nginx_lib_dir;
    $ENV{'LD_LIBRARY_PATH'} = $nginx_lib_dir;
} else {
    # Try Linux .so
    $nginx_lib = "$nginx_lib_dir/libnginx_x402.so";
    if (-f $nginx_lib) {
        $ENV{'LD_LIBRARY_PATH'} = $nginx_lib_dir;
    } else {
        warn "Warning: Library not found. Please build first: cargo build --release --features ngx-rust\n";
    }
}

run_tests();

__DATA__

=== TEST 1: Basic 402 response without payment header
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
--- response_headers
HTTP/1.1 402 Payment Required
--- response_body_like
paymentRequirements|accepts
--- error_code: 402

=== TEST 2: Browser request returns HTML
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36
Accept: text/html,application/xhtml+xml
--- response_headers
Content-Type: text/html
--- response_body_like
<!DOCTYPE html>|Payment Required
--- error_code: 402

=== TEST 3: API client request returns JSON
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
User-Agent: curl/7.68.0
Accept: application/json
--- response_headers
Content-Type: application/json
--- response_body_like
"paymentRequirements"|"accepts"|"error"
--- error_code: 402

=== TEST 4: x402 disabled should pass through
--- config
    location /test {
        x402 off;
        return 200 "OK";
    }
--- request
GET /test
--- response_body
OK
--- error_code: 200

=== TEST 5: Custom facilitator URL
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_facilitator_url https://custom-facilitator.example.com;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
--- error_code: 402

=== TEST 6: Custom description
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_description "Custom payment description";
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
--- error_code: 402

=== TEST 7: Mainnet configuration
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_testnet off;
        return 200 "OK";
    }
--- request
GET /test
--- error_code: 402

=== TEST 8: Custom network
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_network base-sepolia;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
--- error_code: 402

=== TEST 9: Custom resource URL
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_resource /custom/resource;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
--- error_code: 402

=== TEST 10: Different payment amounts
--- config
    location /premium {
        x402 on;
        x402_amount 0.01;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_testnet on;
        return 200 "OK";
    }
    location /standard {
        x402 on;
        x402_amount 0.0001;
        x402_pay_to 0x209693Bc6afc0C5328bA36FaF03C514EF312287C;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /premium
--- error_code: 402
--- request
GET /standard
--- error_code: 402

