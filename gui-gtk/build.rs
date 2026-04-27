// SPDX-License-Identifier: LGPL-2.1-or-later
fn main() {
    // Link against the installed (or locally-built) libopenlawsvpn
    println!("cargo:rustc-link-lib=openlawsvpn");

    // Tell cargo to re-run if the header changes
    println!("cargo:rerun-if-changed=../linux/include/libopenlawsvpn_ffi.h");
    println!("cargo:rerun-if-changed=../linux/src/libopenlawsvpn_ffi.cpp");

    // Search path: env override first (set by RPM build), then local build, then system default
    if let Ok(lib_dir) = std::env::var("OPENLAWSVPN_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    }
    println!("cargo:rustc-link-search=native=../build/lib");

    // Generate FFI bindings from the C extern "C" header
    // We create a thin C header that only exposes the extern "C" symbols
    let bindings = bindgen::Builder::default()
        .header("ffi_wrapper.h")
        .clang_arg("-I../linux/include")
        .allowlist_function("openvpn_client_.*")
        .allowlist_type("Phase1ResultC")
        .allowlist_function("openvpn_free_string")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate FFI bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
