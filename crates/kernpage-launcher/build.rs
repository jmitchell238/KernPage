//! Embeds the app icon (packaging/windows/assets/kernpage.ico) as the exe's icon resource, so the
//! taskbar, Start menu, Explorer and the window title bar show it. Windows targets only; a missing
//! resource compiler is a warning, not a build failure.
fn main() {
    println!("cargo:rerun-if-changed=../../packaging/windows/assets/kernpage.ico");
    println!("cargo:rerun-if-env-changed=KERNPAGE_VERSION");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../packaging/windows/assets/kernpage.ico");
        // Installer test builds (package.ps1 -OverrideVersion) stamp their version into the
        // exe's file properties too, not only into the About box.
        if let Ok(v) = std::env::var("KERNPAGE_VERSION") {
            let n: Vec<u64> = v.split('.').map(|p| p.parse().unwrap_or(0)).collect();
            let packed = (n.first().copied().unwrap_or(0) << 48)
                | (n.get(1).copied().unwrap_or(0) << 32)
                | (n.get(2).copied().unwrap_or(0) << 16);
            res.set("FileVersion", &v);
            res.set("ProductVersion", &v);
            res.set_version_info(winresource::VersionInfo::FILEVERSION, packed);
            res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);
        }
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the app icon: {e}");
        }
    }
}
