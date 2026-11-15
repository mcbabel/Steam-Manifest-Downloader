fn main() {
    println!("cargo:rerun-if-env-changed=SMD_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=SMD_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SMD_BUILD_DATE");
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icons/icon.ico");
        resource.set("ProductName", "Steam Manifest Downloader");
        resource.set("FileDescription", "Steam Manifest Downloader");
        resource.set("LegalCopyright", "MCbabel, GPL-2.0-or-later");
        resource.set("OriginalFilename", "steam-manifest-downloader.exe");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=Windows resources were not embedded: {error}");
        }
    }
}
