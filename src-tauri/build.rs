fn main() {
    println!("cargo:rerun-if-env-changed=SMD_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=SMD_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SMD_BUILD_DATE");

    tauri_build::build()
}
