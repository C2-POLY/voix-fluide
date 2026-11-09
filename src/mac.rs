//! Small macOS helpers: reading a key's state, posting Cmd+V, permissions.

use std::ffi::c_void;

const HID_SYSTEM_STATE: i32 = 1;
const CMD_FLAG: u64 = 0x0010_0000;
const KEY_V: u16 = 9;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
    fn CGEventSourceFlagsState(state: i32) -> u64;
    fn CGEventSourceCreate(state: i32) -> *mut c_void;
    fn CGEventCreateKeyboardEvent(src: *mut c_void, key: u16, down: bool) -> *mut c_void;
    fn CGEventSetFlags(ev: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, ev: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(p: *const c_void);
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// A modifier key that can start dictation on its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShortcutKey {
    pub code: u16,
    /// Device-dependent bit in the modifier flags (NX_DEVICE*KEYMASK).
    pub flag: u64,
    pub label: &'static str,
}

pub const SHORTCUTS: [ShortcutKey; 4] = [
    ShortcutKey { code: 61, flag: 0x40, label: "⌥ Option droite" },
    ShortcutKey { code: 54, flag: 0x10, label: "⌘ Commande droite" },
    ShortcutKey { code: 62, flag: 0x2000, label: "⌃ Contrôle droite" },
    ShortcutKey { code: 63, flag: 0x80_0000, label: "fn" },
];

pub fn key_down(key: ShortcutKey) -> bool {
    unsafe {
        CGEventSourceKeyState(HID_SYSTEM_STATE, key.code)
            || (CGEventSourceFlagsState(HID_SYSTEM_STATE) & key.flag) != 0
    }
}

pub fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

// ---- Microphone permission -------------------------------------------------
// macOS gives an app that has not been granted the microphone a stream of exact
// zeros instead of an error. Whisper then "hears" words such as "you" in it.

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVMediaTypeAudio: *mut c_void;
}

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const std::ffi::c_char) -> *mut c_void;
    fn sel_registerName(name: *const std::ffi::c_char) -> *mut c_void;
    fn objc_msgSend();
}

extern "C" {
    static _NSConcreteGlobalBlock: c_void;
}

/// AVAuthorizationStatus for the microphone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MicAccess {
    NotAsked,
    Restricted,
    Denied,
    Allowed,
}

pub fn mic_access() -> MicAccess {
    unsafe {
        let cls = objc_getClass(c"AVCaptureDevice".as_ptr());
        let sel = sel_registerName(c"authorizationStatusForMediaType:".as_ptr());
        if cls.is_null() {
            return MicAccess::Allowed;
        }
        let f: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> isize =
            std::mem::transmute(objc_msgSend as *const ());
        match f(cls, sel, AVMediaTypeAudio) {
            0 => MicAccess::NotAsked,
            1 => MicAccess::Restricted,
            2 => MicAccess::Denied,
            _ => MicAccess::Allowed,
        }
    }
}

#[repr(C)]
struct BlockDescriptor {
    reserved: usize,
    size: usize,
}

#[repr(C)]
struct Block {
    isa: *const c_void,
    flags: i32,
    reserved: i32,
    invoke: extern "C" fn(*const Block, bool),
    descriptor: *const BlockDescriptor,
}

extern "C" fn on_mic_answer(_block: *const Block, _granted: bool) {}

/// Show macOS's "allow the microphone" prompt (only shown while not asked yet).
pub fn request_mic_access() {
    unsafe {
        let cls = objc_getClass(c"AVCaptureDevice".as_ptr());
        if cls.is_null() {
            return;
        }
        let sel = sel_registerName(c"requestAccessForMediaType:completionHandler:".as_ptr());
        // A global block (no captured variables) that lives for the whole run.
        let desc: &'static BlockDescriptor = Box::leak(Box::new(BlockDescriptor {
            reserved: 0,
            size: std::mem::size_of::<Block>(),
        }));
        let block: &'static Block = Box::leak(Box::new(Block {
            isa: &_NSConcreteGlobalBlock as *const c_void,
            flags: 1 << 28, // BLOCK_IS_GLOBAL
            reserved: 0,
            invoke: on_mic_answer,
            descriptor: desc,
        }));
        let f: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *const Block) =
            std::mem::transmute(objc_msgSend as *const ());
        f(cls, sel, AVMediaTypeAudio, block);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NSRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Top-left corner (in points, origin at the top-left of the primary screen) for a
/// `w`×`h` floating pill centred on the screen the person is working on, `offset`
/// points from its bottom (or top) edge, inside the area not covered by the Dock
/// or the menu bar. Must be called on the main thread.
pub fn overlay_position(w: f32, h: f32, offset: f32, top: bool) -> Option<(f32, f32)> {
    unsafe {
        let cls = objc_getClass(c"NSScreen".as_ptr());
        if cls.is_null() {
            return None;
        }
        let msg_obj: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_rect: extern "C" fn(*mut c_void, *mut c_void) -> NSRect =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_idx: extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let main = msg_obj(cls, sel_registerName(c"mainScreen".as_ptr()));
        let screens = msg_obj(cls, sel_registerName(c"screens".as_ptr()));
        if main.is_null() || screens.is_null() {
            return None;
        }
        let primary = msg_idx(screens, sel_registerName(c"objectAtIndex:".as_ptr()), 0);
        if primary.is_null() {
            return None;
        }
        let primary_h = msg_rect(primary, sel_registerName(c"frame".as_ptr())).h;
        let vf = msg_rect(main, sel_registerName(c"visibleFrame".as_ptr()));
        let x = vf.x + (vf.w - w as f64) / 2.0;
        // Cocoa measures from the bottom of the primary screen; winit from its top.
        let bottom_y = if top {
            vf.y + vf.h - offset as f64 - h as f64
        } else {
            vf.y + offset as f64
        };
        let y = primary_h - (bottom_y + h as f64);
        Some((x as f32, y as f32))
    }
}

pub fn open_microphone_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        .spawn();
}

extern "C" {
    fn responsibility_spawnattrs_setdisclaim(
        attrs: *mut libc::posix_spawnattr_t,
        disclaim: libc::c_int,
    ) -> libc::c_int;
}

const CHILD_ENV: &str = "VOIX_FLUIDE_OWN_PERMISSIONS";

/// When started by another app (Craft Anything, a terminal…), macOS asks *that*
/// app's permission for the microphone and silently feeds zeros if it has none.
/// Start a copy of ourselves that answers for its own permissions, wait for it,
/// and leave with its exit code. Returns only in the copy (or if this fails).
pub fn run_with_own_permissions() {
    if std::env::var_os(CHILD_ENV).is_some() {
        // In the copy: leave with the parent so nothing is left running.
        let parent = unsafe { libc::getppid() };
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if unsafe { libc::getppid() } != parent {
                std::process::exit(0);
            }
        });
        return;
    }
    use std::ffi::CString;
    let Ok(exe) = std::env::current_exe() else { return };
    // Run a signed copy so macOS keeps the permissions across updates.
    let exe = stable_copy(&exe).unwrap_or(exe);
    let Ok(path) = CString::new(exe.as_os_str().as_encoded_bytes()) else { return };
    let args: Vec<CString> = std::env::args_os()
        .filter_map(|a| CString::new(a.as_encoded_bytes()).ok())
        .collect();
    let mut envs: Vec<CString> = std::env::vars_os()
        .filter_map(|(k, v)| {
            let mut kv = k.as_encoded_bytes().to_vec();
            kv.push(b'=');
            kv.extend_from_slice(v.as_encoded_bytes());
            CString::new(kv).ok()
        })
        .collect();
    envs.push(CString::new(format!("{CHILD_ENV}=1")).unwrap());
    let mut argv: Vec<*mut libc::c_char> = args.iter().map(|a| a.as_ptr() as *mut _).collect();
    argv.push(std::ptr::null_mut());
    let mut envp: Vec<*mut libc::c_char> = envs.iter().map(|e| e.as_ptr() as *mut _).collect();
    envp.push(std::ptr::null_mut());
    unsafe {
        let mut attr: libc::posix_spawnattr_t = std::ptr::null_mut();
        if libc::posix_spawnattr_init(&mut attr) != 0 {
            return;
        }
        if responsibility_spawnattrs_setdisclaim(&mut attr, 1) != 0 {
            libc::posix_spawnattr_destroy(&mut attr);
            return;
        }
        let mut pid: libc::pid_t = 0;
        let rc = libc::posix_spawn(
            &mut pid,
            path.as_ptr(),
            std::ptr::null(),
            &attr,
            argv.as_ptr(),
            envp.as_ptr(),
        );
        libc::posix_spawnattr_destroy(&mut attr);
        if rc != 0 {
            return;
        }
        // Pass a stop request from whoever started us on to the copy.
        CHILD_PID.store(pid, std::sync::atomic::Ordering::SeqCst);
        libc::signal(libc::SIGTERM, forward_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, forward_signal as *const () as libc::sighandler_t);
        let mut status = 0;
        while libc::waitpid(pid, &mut status, 0) == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR)
        {}
        let code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { 1 };
        std::process::exit(code);
    }
}

pub const BUNDLE_ID: &str = "app.voixfluide.dictee";

/// macOS remembers the Accessibility and Microphone permissions for a program by
/// its signature. A fresh `cargo build` is signed "ad hoc", tied to that exact
/// build, so every update looks like a new app and the permission given before
/// no longer applies even though its switch still looks on. Keep a copy of the
/// program in Voix Fluide's own folder, signed with the person's Apple signing
/// certificate under a fixed identity, and run that copy. (Signing the running
/// file itself makes macOS drop it, so the build output is never touched.)
fn stable_copy(exe: &std::path::Path) -> Option<std::path::PathBuf> {
    use std::process::Command;
    let dir = crate::whisper::app_dir().join("programme");
    std::fs::create_dir_all(&dir).ok()?;
    let dest = dir.join("voix-fluide");
    let stamp_path = dir.join("source.txt");
    let meta = std::fs::metadata(exe).ok()?;
    let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
    let stamp = format!("{}|{}|{}", exe.display(), meta.len(), mtime);
    if dest.exists() && std::fs::read_to_string(&stamp_path).ok().as_deref() == Some(stamp.as_str()) {
        return Some(dest);
    }
    let tmp = dir.join("voix-fluide.new");
    // A certificate can be revoked or expired: macOS then kills the program at
    // launch. Keep the first one whose signed copy really starts.
    for identity in signing_identities() {
        let _ = std::fs::remove_file(&tmp);
        if std::fs::copy(exe, &tmp).is_err() {
            return None;
        }
        let signed = Command::new("codesign")
            .args(["--force", "--sign", &identity, "--identifier", BUNDLE_ID])
            .arg(&tmp)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let starts = signed
            && Command::new(&tmp)
                .arg(SIGNATURE_CHECK_ARG)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        if starts {
            std::fs::rename(&tmp, &dest).ok()?;
            std::fs::write(&stamp_path, stamp).ok()?;
            return Some(dest);
        }
    }
    let _ = std::fs::remove_file(&tmp);
    None
}

/// Passed to a freshly signed copy: it only checks that macOS lets it start.
pub const SIGNATURE_CHECK_ARG: &str = "--signature-check";

/// Code-signing certificates in the keychain, Developer ID first, then Apple Development.
fn signing_identities() -> Vec<String> {
    let Ok(out) = std::process::Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut ids = Vec::new();
    for kind in ["\"Developer ID Application", "\"Apple Development"] {
        for l in text.lines().filter(|l| l.contains(kind)) {
            if let Some(h) = l.split_whitespace().nth(1) {
                ids.push(h.to_string());
            }
        }
    }
    ids
}

/// Ask macOS to list Voix Fluide under Accessibility and show its prompt.
pub fn request_accessibility() {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        static kAXTrustedCheckOptionPrompt: *const c_void;
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFBooleanTrue: *const c_void;
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
        fn CFDictionaryCreate(
            alloc: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            n: isize,
            kcb: *const c_void,
            vcb: *const c_void,
        ) -> *const c_void;
    }
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        );
        AXIsProcessTrustedWithOptions(dict);
        if !dict.is_null() {
            CFRelease(dict);
        }
    }
}

static CHILD_PID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

extern "C" fn forward_signal(sig: libc::c_int) {
    let pid = CHILD_PID.load(std::sync::atomic::Ordering::SeqCst);
    if pid > 0 {
        unsafe {
            libc::kill(pid, sig);
        }
    }
}

pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// Press Cmd+V in whatever app has the keyboard focus.
pub fn post_paste() {
    unsafe {
        let src = CGEventSourceCreate(HID_SYSTEM_STATE);
        for down in [true, false] {
            let ev = CGEventCreateKeyboardEvent(src, KEY_V, down);
            if !ev.is_null() {
                CGEventSetFlags(ev, CMD_FLAG);
                CGEventPost(0, ev);
                CFRelease(ev);
            }
        }
        if !src.is_null() {
            CFRelease(src);
        }
    }
}

/// Put `text` in the focused field through a temporary clipboard entry,
/// then give the clipboard back its previous text unless `keep` is set.
pub fn paste_text(text: &str, keep: bool, key: ShortcutKey) -> Result<(), String> {
    use std::time::{Duration, Instant};
    // TypingService.waitForPhysicalModifiersToRelease (0x1007ed560): poll every
    // 15 ms; its caller (0x1007e59d8) gives it 2.0 s. If the key is still held,
    // the paste is aborted and the text stays on the clipboard.
    let start = Instant::now();
    while key_down(key) {
        if start.elapsed() >= Duration::from_secs_f64(MODIFIER_RELEASE_TIMEOUT_S) {
            copy_text(text);
            return Err("touche encore enfoncée, texte laissé dans le presse-papiers".into());
        }
        std::thread::sleep(Duration::from_millis(MODIFIER_POLL_MS));
    }
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let previous = cb.get_text().ok();
    cb.set_text(text.to_string()).map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(40));
    post_paste();
    if !keep {
        // PasteDeliveryCoordinator.defaultSettlementDelayNanoseconds (0x1007367e8) = 500 ms,
        // then SystemPasteboardManager.restore(ifUnchangedSince:) (0x100733bc0):
        // only give the old clipboard back if nothing else replaced our text.
        std::thread::sleep(Duration::from_millis(PASTE_SETTLEMENT_MS));
        let unchanged = cb.get_text().map(|t| t == text).unwrap_or(false);
        if let (true, Some(p)) = (unchanged, previous) {
            let _ = cb.set_text(p);
        }
    }
    Ok(())
}

/// 0x1007e59d8: `fmov d0, #2.0` passed as the timeout.
pub const MODIFIER_RELEASE_TIMEOUT_S: f64 = 2.0;
/// 0x1007ed560: `Task.sleep(nanoseconds: 0xE4E1C0)` = 15 ms.
pub const MODIFIER_POLL_MS: u64 = 15;
/// 0x1007367e8: 0x1DCD6500 ns = 500 ms.
pub const PASTE_SETTLEMENT_MS: u64 = 500;

pub fn copy_text(text: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}
