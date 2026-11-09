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
    <key>CFBundleShortVersionString</key><string>0.2.0</string>
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
}
