#ifndef X402_FFI_H
#define X402_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Free a string allocated by x402 functions
 *
 * @param ptr Pointer to string allocated by x402 functions
 */
void x402_free_string(char *ptr);

/**
 * Verify a payment payload
 *
 * @param payment_b64 Base64-encoded payment payload from X-PAYMENT header
 * @param requirements_json JSON string of payment requirements
 * @param facilitator_url URL of the facilitator service (can be NULL for default)
 * @param result Output buffer for result JSON
 * @param result_len Input: buffer size, Output: actual length
 * @return 0 on success (payment is valid), 1 on invalid input, 2 on payment verification failure, 3 on facilitator error, 4 on buffer too small, 5 on internal error
 */
int x402_verify_payment(
    const char *payment_b64,
    const char *requirements_json,
    const char *facilitator_url,
    char *result,
    size_t *result_len
);

/**
 * Create payment requirements JSON
 *
 * @param amount Payment amount as decimal string (e.g., "0.0001")
 * @param pay_to Recipient wallet address
 * @param network Network identifier (e.g., "base-sepolia", can be NULL)
 * @param resource Resource URL (can be NULL for "/")
 * @param description Payment description (can be NULL)
 * @param testnet Whether to use testnet (1 = true, 0 = false)
 * @param result Output buffer for JSON result
 * @param result_len Input: buffer size, Output: actual length
 * @return 0 on success, 1 on invalid input, 4 on buffer too small, 5 on internal error
 */
int x402_create_requirements(
    const char *amount,
    const char *pay_to,
    const char *network,
    const char *resource,
    const char *description,
    int testnet,
    char *result,
    size_t *result_len
);

/**
 * Generate paywall HTML
 *
 * @param requirements_json JSON string of payment requirements
 * @param error_msg Error message to display (can be NULL)
 * @param result Output buffer for HTML
 * @param result_len Input: buffer size, Output: actual length
 * @return 0 on success, 1 on invalid input, 4 on buffer too small, 5 on internal error
 */
int x402_generate_paywall_html(
    const char *requirements_json,
    const char *error_msg,
    char *result,
    size_t *result_len
);

/**
 * Generate JSON 402 response
 *
 * @param requirements_json JSON string of payment requirements
 * @param error_msg Error message (can be NULL)
 * @param result Output buffer for JSON
 * @param result_len Input: buffer size, Output: actual length
 * @return 0 on success, 1 on invalid input, 4 on buffer too small, 5 on internal error
 */
int x402_generate_json_response(
    const char *requirements_json,
    const char *error_msg,
    char *result,
    size_t *result_len
);

/**
 * Check if request is from a browser
 *
 * @param user_agent User-Agent header value (can be NULL)
 * @param accept Accept header value (can be NULL)
 * @return 1 if browser request, 0 if API request
 */
int x402_is_browser_request(
    const char *user_agent,
    const char *accept
);

#ifdef __cplusplus
}
#endif

#endif /* X402_FFI_H */

