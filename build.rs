fn main() {
    // Embed the application icon into the Windows PE binary so it appears in
    // Explorer, the taskbar, and the Alt+Tab switcher.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon/nexperia_x_logo_light.ico");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Could not embed icon resource: {}", e);
        }
    }
}
