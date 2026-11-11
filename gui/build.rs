fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../assets/logo.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        // Only run winres on Windows targets. The icon is committed at
        // assets/logo.ico and bundled by the release workflow.
        if std::path::Path::new("../assets/logo.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("../assets/logo.ico");
            if let Err(e) = res.compile() {
                println!("cargo:warning=Failed to compile Windows resources: {}", e);
            }
        }
    }
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        if let Ok(branch) = String::from_utf8(output.stdout) {
            println!("cargo:rustc-env=GIT_BRANCH={}", branch.trim());
        } else {
            println!("cargo:rustc-env=GIT_BRANCH=unknown");
        }
    } else {
        println!("cargo:rustc-env=GIT_BRANCH=unknown");
    }
}
