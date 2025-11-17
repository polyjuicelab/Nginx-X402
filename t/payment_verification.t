#!/usr/bin/env perl

# Test payment verification with X-PAYMENT header
#
# Note: These tests require a valid payment payload in X-PAYMENT header.
# For now, we test that the module correctly handles the presence/absence
# of the header and returns appropriate responses.

use Test::Nginx::Socket 'no_plan';

repeat_each(1);
log_level('info');

# Find library path
our $nginx_lib_dir = $ENV{'TEST_NGINX_LIB_DIR'} || 'target/aarch64-apple-darwin/release';
our $nginx_lib = "$nginx_lib_dir/libnginx_x402.dylib";

if (-f $nginx_lib) {
    $ENV{'DYLD_LIBRARY_PATH'} = $nginx_lib_dir;
    $ENV{'LD_LIBRARY_PATH'} = $nginx_lib_dir;
} else {
    $nginx_lib = "$nginx_lib_dir/libnginx_x402.so";
    if (-f $nginx_lib) {
        $ENV{'LD_LIBRARY_PATH'} = $nginx_lib_dir;
    }
}

run_tests();

__DATA__

=== TEST 1: Invalid payment header format
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
X-PAYMENT: invalid-base64-format!!!
--- response_headers
HTTP/1.1 402 Payment Required
--- error_code: 402

=== TEST 2: Empty payment header
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
X-PAYMENT: 
--- response_headers
HTTP/1.1 402 Payment Required
--- error_code: 402

=== TEST 3: Missing pay_to configuration
--- config
    location /test {
        x402 on;
        x402_amount 0.0001;
        x402_testnet on;
        return 200 "OK";
    }
--- request
GET /test
--- error_code: 500

