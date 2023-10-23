fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-unknown-linux-musl");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH_x86_64-unknown-linux-musl");

    // Set the PKG_CONFIG_SYSROOT_DIR and PKG_CONFIG_PATH environment variables
    std::env::set_var("PKG_CONFIG_SYSROOT_DIR_x86_64-unknown-linux-musl", "sysroot");
    std::env::set_var("PKG_CONFIG_PATH_x86_64-unknown-linux-musl", "pkg-config");

    // Continue with the rest of your build script
}
