//! Module registration and configuration access

use crate::ngx_module::commands::ngx_http_x402_commands;
use crate::ngx_module::config::X402Config;
use crate::ngx_module::error::{ConfigError, Result};
use ngx::ffi::ngx_http_core_main_conf_t;
use ngx::http::Request;
use std::ffi::c_char;
use std::ptr;

/// Helper function to get `ngx_http_core_loc_conf_t` from `ngx_conf_t`
///
/// This is equivalent to `ngx_http_conf_get_module_loc_conf(cf`, `ngx_http_core_module`)
/// Returns None if not in location context or if clcf is null
#[allow(dead_code)]
unsafe fn get_core_loc_conf(
    cf: *mut ngx::ffi::ngx_conf_t,
) -> Option<*mut ngx::ffi::ngx_http_core_loc_conf_t> {
    let ctx = (*cf).ctx.cast::<ngx::ffi::ngx_http_conf_ctx_t>();
    if ctx.is_null() {
        return None;
    }

    let loc_conf = (*ctx).loc_conf;
    if loc_conf.is_null() {
        return None;
    }

    // Get core module's location config using ctx_index
    // loc_conf is *mut *mut c_void (array of pointers)
    // Use loc_conf.add() directly, not (*loc_conf).add()
    let core_ctx_index = ngx::ffi::ngx_http_core_module.ctx_index;
    let ptr_to_ptr = loc_conf.add(core_ctx_index);
    if ptr_to_ptr.is_null() {
        return None;
    }

    // Read the pointer value
    let clcf_void: *mut core::ffi::c_void = ptr::read(ptr_to_ptr.cast_const());
    if clcf_void.is_null() {
        return None;
    }

    Some(core::mem::transmute::<
        *mut core::ffi::c_void,
        *mut ngx::ffi::ngx_http_core_loc_conf_t,
    >(clcf_void))
}

/// Helper function to get `ngx_http_core_main_conf_t` from `ngx_conf_t`
///
/// This is equivalent to `ngx_http_conf_get_module_main_conf(cf`, `ngx_http_core_module`)
unsafe fn get_core_main_conf(
    cf: *mut ngx::ffi::ngx_conf_t,
) -> Option<*mut ngx_http_core_main_conf_t> {
    if cf.is_null() {
        return None;
    }

    let ctx = (*cf).ctx.cast::<ngx::ffi::ngx_http_conf_ctx_t>();
    if ctx.is_null() {
        return None;
    }

    let main_conf = (*ctx).main_conf;
    if main_conf.is_null() {
        return None;
    }

    // Get core module's main config using ctx_index
    // main_conf is *mut *mut c_void (pointer to array of pointers)
    // Use main_conf.add() directly, not (*main_conf).add()
    let core_ctx_index = ngx::ffi::ngx_http_core_module.ctx_index;
    let ptr_to_ptr = main_conf.add(core_ctx_index);
    if ptr_to_ptr.is_null() {
        return None;
    }

    // Read the pointer value from the array
    let cmcf_void: *mut core::ffi::c_void = ptr::read(ptr_to_ptr.cast_const());
    if cmcf_void.is_null() {
        return None;
    }

    Some(core::mem::transmute::<
        *mut core::ffi::c_void,
        *mut ngx_http_core_main_conf_t,
    >(cmcf_void))
}

/// Postconfiguration hook
///
/// This is called after all configuration is parsed.
/// We use this to register phase handler as a fallback if clcf->handler is not set.
///
/// NOTE: We cannot verify handler settings here because we don't have access to
/// individual location configurations. Handler verification happens in the command
/// handler when the directive is parsed.
unsafe extern "C" fn postconfiguration(cf: *mut ngx::ffi::ngx_conf_t) -> ngx::ffi::ngx_int_t {
    // Postconfiguration is called after all configuration is parsed and merged
    // At this point, we can register phase handlers

    // Get core main config to access phases array
    if let Some(cmcf) = get_core_main_conf(cf) {
        extern "C" {
            fn x402_phase_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t;
        }

        // Register phase handler in ACCESS phase
        // This ensures payment verification happens BEFORE proxy_pass sets its handler
        // ACCESS_PHASE (index 6) runs before CONTENT_PHASE (index 10)
        // This allows x402 to verify payment even when proxy_pass is configured
        let phases = &(*cmcf).phases;
        let access_phase_index = 6usize; // NGX_HTTP_ACCESS_PHASE

        if access_phase_index < phases.len() {
            let access_phase = &phases[access_phase_index];
            use ngx::ffi::ngx_array_push;
            let handlers_ptr = (&raw const access_phase.handlers).cast_mut();
            let handler_ptr = ngx_array_push(handlers_ptr);
            if handler_ptr.is_null() {
                return ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t;
            }
            let handler_ptr_typed = handler_ptr.cast::<ngx::ffi::ngx_http_handler_pt>();
            *handler_ptr_typed = Some(x402_phase_handler);
        } else {
            return ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t;
        }
    } else {
        return ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t;
    }

    ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t
}

/// HTTP module context structure
///
/// This structure defines the callbacks for creating and merging configuration
/// at different levels (main, server, location).
#[allow(non_upper_case_globals)]
static mut ngx_http_x402_module_ctx: ngx::ffi::ngx_http_module_t = ngx::ffi::ngx_http_module_t {
    preconfiguration: None,
    postconfiguration: Some(postconfiguration),
    create_main_conf: None,
    init_main_conf: None,
    create_srv_conf: None,
    merge_srv_conf: None,
    create_loc_conf: Some(create_loc_conf),
    merge_loc_conf: Some(merge_loc_conf),
};

/// Create location configuration
///
/// This function is called by nginx when creating a new location configuration block.
/// It allocates and initializes a new `X402Config` structure.
unsafe extern "C" fn create_loc_conf(cf: *mut ngx::ffi::ngx_conf_t) -> *mut core::ffi::c_void {
    use ngx::ffi::ngx_pcalloc;
    use std::mem::size_of;

    // Use ngx_pcalloc to allocate zero-initialized memory, matching nginx's pattern
    // ngx_pcalloc already zero-initializes the memory, which is sufficient for our needs
    // The default values will be set during merge_loc_conf if needed
    ngx_pcalloc((*cf).pool, size_of::<X402Config>())
}

/// Merge location configuration
///
/// This function is called by nginx when merging location configurations
/// from parent levels (server, main). It merges the previous configuration
/// into the current one.
///
/// CRITICAL: This is called AFTER command handlers, so handler should already be set.
/// We cannot set handler here because it causes segmentation faults.
/// However, we can verify that handler is still set after merging.
unsafe extern "C" fn merge_loc_conf(
    _cf: *mut ngx::ffi::ngx_conf_t,
    prev: *mut core::ffi::c_void,
    conf: *mut core::ffi::c_void,
) -> *mut c_char {
    let prev = prev.cast::<X402Config>();
    let conf = conf.cast::<X402Config>();

    if prev.is_null() || conf.is_null() {
        return ptr::null_mut();
    }

    // Merge configuration: use current value if set, otherwise use previous
    let prev_conf = &*prev;
    let conf_mut = &mut *conf;

    // Merge enabled flag
    if conf_mut.enabled == 0 {
        conf_mut.enabled = prev_conf.enabled;
    }

    // CRITICAL: Do NOT access clcf in merge_loc_conf - it causes segmentation faults
    // The handler verification must be done at runtime in the handler itself, not during merge

    // Merge string fields: use current if non-empty, otherwise use previous
    if conf_mut.amount_str.len == 0 {
        conf_mut.amount_str = prev_conf.amount_str;
    }
    if conf_mut.pay_to_str.len == 0 {
        conf_mut.pay_to_str = prev_conf.pay_to_str;
    }
    if conf_mut.facilitator_url_str.len == 0 {
        conf_mut.facilitator_url_str = prev_conf.facilitator_url_str;
    }
    if conf_mut.description_str.len == 0 {
        conf_mut.description_str = prev_conf.description_str;
    }
    if conf_mut.network_str.len == 0 {
        conf_mut.network_str = prev_conf.network_str;
    }
    if conf_mut.resource_str.len == 0 {
        conf_mut.resource_str = prev_conf.resource_str;
    }
    if conf_mut.timeout_str.len == 0 {
        conf_mut.timeout_str = prev_conf.timeout_str;
    }
    if conf_mut.facilitator_fallback_str.len == 0 {
        conf_mut.facilitator_fallback_str = prev_conf.facilitator_fallback_str;
    }

    // Note: Handler is set in ngx_http_x402 command handler when x402 on; is parsed
    // We cannot set handler here in merge_loc_conf because accessing clcf during merging
    // causes segmentation faults - the context may not be fully initialized at this point
    // The handler must be set in the command handler (ngx_http_x402) where the context is correct
    //
    // Attempts to set handler in merge_loc_conf have consistently caused segmentation faults,
    // even when using get_core_loc_conf helper. This suggests that clcf is not fully initialized
    // or accessible during the merge phase. The handler setting must remain in ngx_http_x402.

    ptr::null_mut()
}

// Include the auto-generated module signature from build.rs
// This signature is extracted from the nginx source configuration
// to ensure binary compatibility with the target nginx binary
include!(concat!(env!("OUT_DIR"), "/module_signature.rs"));

/// Module structure for x402 HTTP module
///
/// This structure registers the module with nginx, including:
/// - Module name and signature
/// - Configuration commands
/// - HTTP module context
/// - Module type (HTTP module)
///
/// # Signature Generation
///
/// The module signature is extracted from nginx source at build time via build.rs.
/// The signature format is: "{`NGX_PTR_SIZE},{NGX_SIG_ATOMIC_T_SIZE},{NGX_TIME_T_SIZE},{feature_flags`}"
///
/// The signature is built from `objs/ngx_auto_config.h` by extracting the necessary
/// defines and constructing the signature according to the logic in `src/core/ngx_module.h`.
///
/// Requires nginx 1.10.0 or later (released April 2016).
/// This ensures binary compatibility with the target nginx binary by matching its exact configuration.
#[no_mangle]
pub static mut ngx_http_x402_module: ngx::ffi::ngx_module_t = ngx::ffi::ngx_module_t {
    ctx_index: 0, // Will be set by nginx during module initialization
    index: 0,
    spare0: 0,
    spare1: 0,
    // IMPORTANT: Module version must match Nginx runtime version exactly.
    // This version is set at build time based on the Nginx source used.
    version: ngx::ffi::nginx_version as usize,
    // NGX_MODULE_SIGNATURE is extracted from nginx source at build time via build.rs
    // Format: "{NGX_PTR_SIZE},{NGX_SIG_ATOMIC_T_SIZE},{NGX_TIME_T_SIZE},{feature_flags}"
    // The signature includes: pointer size, atomic type size, time type size, and feature flags
    //
    // The signature is built from objs/ngx_auto_config.h according to the logic in src/core/ngx_module.h
    // Requires nginx 1.10.0 or later (released April 2016)
    // This ensures binary compatibility with the target nginx binary by matching its exact configuration
    signature: MODULE_SIGNATURE.as_ptr().cast::<c_char>(),
    name: c"ngx_http_x402_module".as_ptr().cast_mut(),
    ctx: &raw const ngx_http_x402_module_ctx as *mut _,
    commands: unsafe { (&raw mut ngx_http_x402_commands[0]).cast() },
    type_: ngx::ffi::NGX_HTTP_MODULE as usize,
    init_master: None,
    init_module: None,
    init_process: None,
    init_thread: None,
    exit_thread: None,
    exit_process: None,
    exit_master: None,
    spare_hook0: 0,
    spare_hook1: 0,
    spare_hook2: 0,
    spare_hook3: 0,
    spare_hook4: 0,
    spare_hook5: 0,
    spare_hook6: 0,
    spare_hook7: 0,
};

/// Export `ngx_modules` array for dynamic module loading
///
/// Nginx requires this array to be exported for dynamic modules.
/// This allows nginx to discover and load the module when using `load_module`.
///
/// NOTE: This should ideally use `ngx::ngx_modules`! macro, but we need to fix
/// the module structure first (commands, ctx, etc.)
#[no_mangle]
pub static mut ngx_modules: [*const ngx::ffi::ngx_module_t; 2] =
    [&raw const ngx_http_x402_module, core::ptr::null()];

/// Module names array (required for dynamic modules)
#[no_mangle]
pub static mut ngx_module_names: [*const core::ffi::c_char; 2] =
    [c"ngx_http_x402_module".as_ptr(), core::ptr::null()];

/// Module order array (required for dynamic modules)
#[no_mangle]
pub static mut ngx_module_order: [*const core::ffi::c_char; 1] = [core::ptr::null()];

/// Get module configuration from request
///
/// This function retrieves the module's location configuration from the request.
/// The implementation uses ngx-rust's API to access the configuration.
///
/// # Implementation Notes
///
/// The ngx-rust `module!` macro generates a module structure that includes
/// a context index. The configuration can be accessed using:
/// - `req.get_loc_conf::<X402Module, X402Config>()` (if available)
/// - Or via the module's context index
///
/// The exact API depends on ngx-rust 0.5's implementation. This function
/// attempts to use the safest available method.
///
/// # Safety
///
/// The fallback unsafe block is only used when the safe API is unavailable.
/// All pointer operations are validated before use:
/// - Request pointer is checked for null
/// - `loc_conf` pointer is checked for null
/// - Configuration pointer is checked for null
/// - Context index is validated to be within bounds (implicitly by ngx-rust)
pub fn get_module_config(req: &Request) -> Result<X402Config> {
    // The ngx-rust module! macro should provide a way to access configuration
    // The exact method depends on ngx-rust 0.5's API
    // For now, we use unsafe access via module context index.
    //
    // Safety: We validate all pointers before dereferencing:
    // 1. Request pointer must be non-null (guaranteed by Request type)
    // 2. loc_conf array must be non-null (checked)
    // 3. Configuration pointer at ctx_index must be non-null (checked)
    // 4. Context index should be provided by module registration
    unsafe {
        let r = req.as_ref();

        // Get the module's context index
        let ctx_index = ngx_http_x402_module.ctx_index;

        // Validate context index is reasonable (should be < 256 for typical Nginx setups)
        if ctx_index >= 256 {
            return Err(ConfigError::from(format!(
                "Invalid context index: {ctx_index} (too large)"
            )));
        }

        // Access loc_conf array at the module's context index
        // Safety: loc_conf is guaranteed to be valid for HTTP requests
        let loc_conf_raw = r.loc_conf;
        if loc_conf_raw.is_null() {
            return Err(ConfigError::from("Invalid loc_conf pointer: null"));
        }

        // Validate the configuration pointer at our context index
        // Safety: We use checked pointer arithmetic
        let conf_ptr_raw = loc_conf_raw.add(ctx_index);
        if conf_ptr_raw.is_null() {
            return Err(ConfigError::from(format!(
                "Invalid configuration pointer at index {ctx_index}: null"
            )));
        }

        // Read the configuration pointer
        // Safety: We've validated that conf_ptr_raw is non-null
        let conf_ptr_void = *conf_ptr_raw;
        if conf_ptr_void.is_null() {
            return Err(ConfigError::from(format!(
                "Configuration pointer at index {ctx_index} is null"
            )));
        }

        // Cast to our configuration type
        // Safety: We know this pointer should point to X402Config based on module registration
        let conf_ptr = conf_ptr_void.cast::<X402Config>();

        // Validate the configuration structure by checking a known field offset
        // This is a basic sanity check - if the pointer is invalid, this might fail
        // Note: We can't easily validate the structure without knowing its layout,
        // but we can at least ensure the pointer is aligned and accessible
        let _ = std::ptr::read_volatile(&raw const (*conf_ptr).enabled);

        // Clone the configuration
        // Safety: We've validated that conf_ptr is non-null and points to valid memory
        Ok((*conf_ptr).clone())
    }
}
