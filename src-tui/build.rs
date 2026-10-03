fn main() {
    println!("cargo:rerun-if-changed=../src-tauri/tauri.conf.json");
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed=../public/locales");
    println!("cargo:rerun-if-env-changed=SMD_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=SMD_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SMD_BUILD_DATE");

    let version = std::fs::read_to_string("../src-tauri/tauri.conf.json")
        .ok()
        .and_then(|conf| {
            let after = conf.split("\"version\"").nth(1)?;
            let start = after.find('"')? + 1;
            let rest = &after[start..];
            Some(rest[..rest.find('"')?].to_string())
        })
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_default());
    println!("cargo:rustc-env=SMD_APP_VERSION={}", version);
}
