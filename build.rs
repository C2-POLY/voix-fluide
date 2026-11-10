// Embed an Info.plist in the program so macOS knows its name and why it asks
// for the microphone (without it the microphone silently gives silence).
fn main() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("Info.plist");
    let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key><string>app.voixfluide.dictee</string>
    <key>CFBundleName</key><string>Voix Fluide</string>
    <key>CFBundleDisplayName</key><string>Voix Fluide</string>
    <key>CFBundleShortVersionString</key><string>0.3.0</string>
    <key>NSMicrophoneUsageDescription</key><string>Voix Fluide utilise le micro pour transcrire votre voix en texte, entièrement sur ce Mac.</string>
    <key>NSAccessibilityUsageDescription</key><string>Voix Fluide colle le texte dicté dans l'application active.</string>
</dict>
</plist>
"#;
    std::fs::write(&out, plist).unwrap();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!(
            "cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            out.display()
        );
    }
    println!("cargo:rerun-if-changed=build.rs");
    embed_build_info();
}

// Embed the commit (VF_COMMIT) and whether this is a CI release build
// (VF_RELEASE), so the app can tell builds of the same version apart.
fn embed_build_info() {
    let git = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("git").args(args).output().ok()?;
        let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
        (out.status.success() && !s.is_empty()).then_some(s)
    };
    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| git(&["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".into());
    let release = if std::env::var("VF_RELEASE").as_deref() == Ok("1") { "1" } else { "0" };
    println!("cargo:rustc-env=VF_COMMIT={commit}");
    println!("cargo:rustc-env=VF_RELEASE={release}");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=VF_RELEASE");

    // Rebuild when HEAD moves: watch HEAD (in a worktree's own git dir, since
    // its .git is a file), the branch it points to, and packed-refs.
    let watch = |path: String| {
        if std::path::Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    };
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        watch(head);
    }
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        if let Some(path) = git(&["rev-parse", "--git-path", &branch]) {
            watch(path);
        }
        if let Some(packed) = git(&["rev-parse", "--git-path", "packed-refs"]) {
            watch(packed);
        }
    }
}
