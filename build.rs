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

    // For dynamic nginx modules, we need to allow undefined symbols
    // because nginx symbols will be resolved at runtime when the module is loaded by nginx
    //
    // macOS requires explicit flags to allow undefined symbols
    // Linux shared libraries (.so) allow undefined symbols by default
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }

    // Linux doesn't need special flags - shared libraries allow undefined symbols by default
    // The symbols will be resolved when nginx loads the module at runtime

    // Apply compiler/linker flags from environment for binary compatibility
    // These flags are extracted from system nginx configure arguments in postinst/%post scripts
    // They ensure the module is compiled with the same options as system nginx
    if let Ok(rustflags) = std::env::var("RUSTFLAGS") {
        // RUSTFLAGS is already set by the build script, cargo will use it automatically
        // We just log it here for debugging
        println!(
            "cargo:warning=Using RUSTFLAGS from environment: {}",
            rustflags
        );
    }
}
