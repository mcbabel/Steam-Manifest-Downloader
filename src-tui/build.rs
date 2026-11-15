fn main() {
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed=../locales");
    println!("cargo:rerun-if-env-changed=SMD_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=SMD_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SMD_BUILD_DATE");
    println!("cargo:rustc-env=SMD_APP_VERSION={}", std::env::var("CARGO_PKG_VERSION").unwrap_or_default());
}
