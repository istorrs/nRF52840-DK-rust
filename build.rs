use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Determine which memory layout to use based on features
    let memory_file = if cfg!(feature = "ble") {
        // Use SoftDevice memory layout for BLE app
        "memory-softdevice.x"
    } else if cfg!(feature = "gpio") {
        // Use SoftDevice-preserving layout for GPIO app
        "memory-gpio-with-softdevice.x"
    } else {
        // Default to full memory layout (legacy)
        "memory-no-softdevice.x"
    };

    // Copy the appropriate memory file to the output directory
    fs::copy(memory_file, out_dir.join("memory.x")).unwrap();

    // Tell cargo to rerun this build script if memory files change
    println!("cargo:rerun-if-changed=memory-no-softdevice.x");
    println!("cargo:rerun-if-changed=memory-softdevice.x");
    println!("cargo:rerun-if-changed=build.rs");

    // Tell cargo to look in the output directory for linker scripts
    println!("cargo:rustc-link-search={}", out_dir.display());

    // CRITICAL: Add --nmagic linker argument (link.x and defmt.x already in config.toml)
    println!("cargo:rustc-link-arg-bins=--nmagic");

    // Print which memory layout is being used for debugging
    println!("cargo:warning=Using memory layout: {}", memory_file);
}
