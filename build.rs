use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    println!("cargo:rerun-if-changed=assets/app.ico");

    // Embed the app icon into the executable (Explorer, taskbar, Alt-Tab).
    // Requires windres on the GNU toolchain; on MSVC (GitHub Actions) rc.exe
    // is available. Missing tooling only skips the icon — never fails the build.
    match winresource::WindowsResource::new()
        .set_icon("assets/app.ico")
        .compile()
    {
        Ok(()) => {}
        Err(e) => println!("cargo:warning=exe icon not embedded (toolchain lacks a resource compiler): {e}"),
    }
}
