fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Note: Module signature is generated dynamically in module.rs using
    // ngx::ffi constants (NGX_PTR_SIZE, NGX_SIG_ATOMIC_T_SIZE, NGX_TIME_T_SIZE,
    // and NGX_MODULE_SIGNATURE_1 through NGX_MODULE_SIGNATURE_N).
    //
    // The signature is constructed at compile time using a const function that
    // reads these constants from ngx::ffi. This ensures the signature matches
    // the nginx binary built with the same source, without hardcoding.
    //
    // See src/ngx_module/module.rs for the implementation.
}
