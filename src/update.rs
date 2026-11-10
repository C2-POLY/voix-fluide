//! Updates: read the latest GitHub Release, download it, check it, install it and
//! relaunch. Run from `Voix Fluide.app`, it replaces the app with the one of the
//! Release's `.dmg`; run from a bare binary (0.3.0 and older, `cargo run`), it puts
//! the Release's binary in `programme/source/` and relaunches from it.

use crate::engine::{Phase, SessionState, SharedRef};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const LATEST_JSON: &str = "https://github.com/C2-POLY/voix-fluide/releases/latest/download/latest.json";
const LATEST_BINARY: &str = "https://github.com/C2-POLY/voix-fluide/releases/latest/download/voix-fluide-aarch64-apple-darwin";
const LATEST_DMG: &str = "https://github.com/C2-POLY/voix-fluide/releases/latest/download/Voix-Fluide.dmg";

/// What the Updates page shows.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading,
    Failed(String),
}

pub type UpdateRef = Arc<Mutex<UpdateState>>;

/// The fields of the Release's `latest.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub commit: String,
    /// SHA-256 of the bare binary (what 0.3.0 and older install).
    pub sha256: String,
    /// SHA-256 of `Voix-Fluide.dmg`, absent from Releases made before the .dmg.
    pub dmg_sha256: Option<String>,
}

/// Read `latest.json`: `{"version", "commit", "sha256"}`, all non-empty strings,
/// and `dmg_sha256` when the Release has a .dmg.
pub fn parse_release(text: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|_| "Réponse de GitHub illisible.".to_string())?;
    let field = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .ok_or_else(|| format!("Réponse de GitHub incomplète (« {k} » manque)."))
    };
    Ok(Release {
        version: field("version")?,
        commit: field("commit")?,
        sha256: field("sha256")?,
        dmg_sha256: field("dmg_sha256").ok(),
    })
}

/// A Release is newer when its commit differs from ours (tags move during the day,
/// so the version alone says nothing). A build without a known commit always differs.
pub fn update_available(local: &str, remote: &str) -> bool {
    local == "unknown" || !local.eq_ignore_ascii_case(remote)
}

/// Only Release builds check at launch, and never while taking a screenshot.
pub fn check_at_launch(release: bool, screenshot: bool) -> bool {
    release && !screenshot
}

/// Why "Installer" must wait, or None when it can run now.
pub fn install_blocked(phase: Phase, running_sessions: usize) -> Option<&'static str> {
    if phase != Phase::Idle {
        Some("Installation possible à la fin de la dictée.")
    } else if running_sessions > 0 {
        Some("Installation possible à la fin de la mini-session Claude Code.")
    } else {
        None
    }
}

/// VF_UPDATE=<state> (screenshots): a fake state, no network.
pub fn fake_state(name: &str) -> Option<UpdateState> {
    Some(match name {
        "checking" => UpdateState::Checking,
        "uptodate" => UpdateState::UpToDate,
        "available" => UpdateState::Available(Release {
            version: env!("CARGO_PKG_VERSION").into(),
            commit: "abc1234def5678901234567890abcdef12345678".into(),
            sha256: String::new(),
            dmg_sha256: None,
        }),
        "downloading" => UpdateState::Downloading,
        "failed" => UpdateState::Failed("GitHub ne répond pas : vérifiez la connexion.".into()),
        _ => return None,
    })
}

/// A French message for a failed request.
fn http_error(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(404, _) => "Aucune Release publiée sur GitHub.".into(),
        ureq::Error::Status(code, _) => format!("GitHub a répondu {code}."),
        ureq::Error::Transport(_) => "GitHub ne répond pas : vérifiez la connexion.".into(),
    }
}

/// Fetch and read the latest Release's `latest.json` (10 s at most).
pub fn check() -> Result<Release, String> {
    let resp = ureq::get(LATEST_JSON).timeout(Duration::from_secs(10)).call().map_err(http_error)?;
    let text = resp.into_string().map_err(|_| "GitHub ne répond pas : vérifiez la connexion.".to_string())?;
    parse_release(&text)
}

/// SHA-256 of a file, in lowercase hex, through `shasum`.
pub fn sha256_of(path: &Path) -> Result<String, String> {
    let out = std::process::Command::new("/usr/bin/shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .map_err(|e| format!("shasum : {e}"))?;
    if !out.status.success() {
        return Err("shasum a échoué.".into());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().next().map(str::to_lowercase).ok_or_else(|| "shasum a échoué.".into())
}

/// Whether a file's SHA-256 is the expected one (case does not matter).
pub fn sha256_matches(path: &Path, expected: &str) -> Result<bool, String> {
    Ok(sha256_of(path)?.eq_ignore_ascii_case(expected.trim()))
}

/// `programme/source/`: where the downloaded program lives.
fn source_dir() -> PathBuf {
    crate::whisper::app_dir().join("programme").join("source")
}

/// The installed program the relaunch starts.
pub fn installed_path() -> PathBuf {
    source_dir().join("voix-fluide")
}

/// Whether the program starts and answers `--version` (10 s at most).
fn starts(path: &Path) -> bool {
    let Ok(mut child) = std::process::Command::new(path)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    for _ in 0..100 {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => return false,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    false
}

/// Download the Release binary to `voix-fluide.part`, check it, and rename it to
/// `voix-fluide`. On any failure `.part` is removed and nothing else changes.
pub fn install(release: &Release) -> Result<PathBuf, String> {
    let dir = source_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let part = dir.join("voix-fluide.part");
    if let Err(e) = download_and_check(&part, &release.sha256) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    let dest = installed_path();
    std::fs::rename(&part, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        format!("Installation impossible : {e}")
    })?;
    Ok(dest)
}

fn download_and_check(part: &Path, sha256: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    download(LATEST_BINARY, part)?;
    if !sha256_matches(part, sha256)? {
        return Err("Le fichier téléchargé ne correspond pas à la Release (SHA-256).".into());
    }
    std::fs::set_permissions(part, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    if !starts(part) {
        return Err("Le programme téléchargé ne démarre pas.".into());
    }
    Ok(())
}

/// Download `url` to `to`.
fn download(url: &str, to: &Path) -> Result<(), String> {
    // Tens of MB: a connection timeout, then a read timeout instead of a total one.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .build();
    let resp = agent.get(url).call().map_err(http_error)?;
    let mut out = std::fs::File::create(to).map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = reader.read(&mut buf).map_err(|_| "Téléchargement interrompu.".to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

/// The program the launcher was started from: the copy in `programme/` gets it from
/// the launcher (`mac::LAUNCHER_ENV`), otherwise it is this program.
pub fn launcher_exe() -> Option<PathBuf> {
    std::env::var_os(crate::mac::LAUNCHER_ENV)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
}

/// The `.app` a program runs from: `X.app/Contents/MacOS/<exe>` gives `X.app`.
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let is_app = app.extension().is_some_and(|x| x == "app");
    (macos.file_name()? == "MacOS" && contents.file_name()? == "Contents" && is_app).then(|| app.to_path_buf())
}

/// The installed `Voix Fluide.app`, when Voix Fluide was started from one.
pub fn running_bundle() -> Option<PathBuf> {
    bundle_of(&launcher_exe()?)
}

/// Why the app at `bundle` cannot be replaced where it is, from its path alone.
pub fn bundle_place_refused(bundle: &Path) -> Option<String> {
    if bundle.components().any(|c| c.as_os_str() == "AppTranslocation") {
        return Some("macOS fait tourner Voix Fluide depuis une copie temporaire. Glissez Voix Fluide.app dans Applications, ouvrez-la depuis là, puis mettez à jour.".into());
    }
    if bundle.starts_with("/Volumes") && bundle.parent().is_some_and(|d| d.parent() == Some(Path::new("/Volumes"))) {
        return Some("Voix Fluide tourne depuis l'image disque. Glissez Voix Fluide.app dans Applications, ouvrez-la depuis là, puis mettez à jour.".into());
    }
    None
}

/// A path removed (file or folder) when dropped.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0).or_else(|_| std::fs::remove_file(&self.0));
    }
}

/// Run a tool; on failure, `what` and its error output.
fn run(cmd: &mut std::process::Command, what: &str) -> Result<(), String> {
    let out = cmd.stdin(std::process::Stdio::null()).output().map_err(|e| format!("{what} : {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("{what} : {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Mount `dmg` read-only on `mount`, copy its `Voix Fluide.app` to `to` and always unmount.
fn copy_from_dmg(dmg: &Path, mount: &Path, to: &Path) -> Result<(), String> {
    use std::process::Command;
    std::fs::create_dir_all(mount).map_err(|e| e.to_string())?;
    run(
        Command::new("/usr/bin/hdiutil").args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint"]).arg(mount).arg(dmg),
        "Ouverture de l'image disque impossible",
    )?;
    let app = mount.join("Voix Fluide.app");
    let copied = if app.is_dir() {
        run(Command::new("/usr/bin/ditto").arg(&app).arg(to), "Copie de la nouvelle app impossible")
    } else {
        Err("L'image disque ne contient pas Voix Fluide.app.".into())
    };
    if run(Command::new("/usr/bin/hdiutil").arg("detach").arg(mount), "hdiutil detach").is_err() {
        let _ = run(Command::new("/usr/bin/hdiutil").args(["detach", "-force"]).arg(mount), "hdiutil detach");
    }
    copied
}

/// Replace the app at `bundle` with the one of the Release's `.dmg`. On any failure
/// the installed app is left as it was (or put back) and the downloaded files removed.
pub fn install_dmg(release: &Release, bundle: &Path) -> Result<(), String> {
    let Some(want) = &release.dmg_sha256 else {
        return Err("Cette Release n'a pas de .dmg vérifiable : téléchargez-la depuis GitHub.".into());
    };
    if let Some(why) = bundle_place_refused(bundle) {
        return Err(why);
    }
    let dir = bundle.parent().ok_or("L'app n'a pas de dossier.")?;
    let name = bundle.file_name().ok_or("L'app n'a pas de nom.")?.to_string_lossy().into_owned();
    let pid = std::process::id();
    // Hidden names next to the app, on its volume, so the swap is two renames.
    let staged = dir.join(format!(".{name}.update-{pid}"));
    let backup = dir.join(format!(".{name}.old-{pid}"));
    let _ = std::fs::remove_dir_all(&staged);
    let _ = std::fs::remove_dir_all(&backup);
    if let Err(e) = std::fs::create_dir(&staged).and_then(|_| std::fs::remove_dir(&staged)) {
        return Err(format!("Voix Fluide ne peut pas écrire dans {} ({e}). Téléchargez la nouvelle version depuis GitHub et glissez-la dans Applications.", dir.display()));
    }

    let tmp = std::env::temp_dir().join(format!("voix-fluide-update-{pid}"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let _tmp = Cleanup(tmp.clone());
    let dmg = tmp.join("Voix-Fluide.dmg");
    download(LATEST_DMG, &dmg)?;
    if !sha256_matches(&dmg, want)? {
        return Err("Le fichier téléchargé ne correspond pas à la Release (SHA-256).".into());
    }
    replace_app(&dmg, &tmp.join("mount"), bundle, &staged, &backup)
}

/// Copy the app of `dmg` to `staged`, check it, and swap it with `bundle` by two renames.
fn replace_app(dmg: &Path, mount: &Path, bundle: &Path, staged: &Path, backup: &Path) -> Result<(), String> {
    use std::process::Command;
    let staged_guard = Cleanup(staged.to_path_buf());
    copy_from_dmg(dmg, mount, staged)?;
    if !staged.join("Contents/MacOS/voix-fluide").is_file() {
        return Err("La nouvelle app est incomplète.".into());
    }
    run(Command::new("/usr/bin/codesign").args(["--verify", "--deep", "--strict"]).arg(staged), "La signature de la nouvelle app n'est pas valide")?;

    std::fs::rename(bundle, backup).map_err(|e| format!("Impossible de mettre l'app actuelle de côté : {e}"))?;
    if let Err(e) = std::fs::rename(staged, bundle) {
        return Err(match std::fs::rename(backup, bundle) {
            Ok(()) => format!("Impossible de mettre la nouvelle app en place : {e}"),
            Err(r) => format!("Impossible de mettre la nouvelle app en place ({e}) ni de remettre l'ancienne ({r}) : elle est dans {}", backup.display()),
        });
    }
    drop(staged_guard);
    let _ = std::fs::remove_dir_all(backup);
    Ok(())
}

/// Open `bundle` again once the launcher has quit, from a detached shell, then quit.
/// The launcher (our parent when we are the copy in `programme/`) leaves after us.
pub fn relaunch_app(bundle: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    const SCRIPT: &str = r#"i=0; while kill -0 "$1" 2>/dev/null && [ "$i" -lt 100 ]; do sleep 0.2; i=$((i+1)); done; exec /usr/bin/open -n "$2""#;
    let wait_for = if std::env::var_os(crate::mac::CHILD_ENV).is_some() { unsafe { libc::getppid() } } else { std::process::id() as libc::pid_t };
    let mut cmd = std::process::Command::new("/bin/sh");
    cmd.args(["-c", SCRIPT, "sh"])
        .arg(wait_for.to_string())
        .arg(bundle)
        .env_remove(crate::mac::CHILD_ENV)
        .env_remove(crate::mac::LAUNCHER_ENV)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // Its own session: it must outlive us and the launcher.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn().map_err(|e| format!("Relance impossible : {e}"))?;
    std::process::exit(0);
}

/// Start the installed program as a fresh launcher (without the copy's marker, so it
/// goes through `run_with_own_permissions` and re-signs), detached, then quit.
pub fn relaunch(path: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let mut cmd = std::process::Command::new(path);
    cmd.env_remove(crate::mac::CHILD_ENV)
        .env_remove(crate::mac::LAUNCHER_ENV)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // Its own session: it must outlive us and the terminal that started us.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn().map_err(|e| format!("Relance impossible : {e}"))?;
    // The waiting launcher above us then leaves with this code.
    std::process::exit(0);
}

/// Check in a thread; the window repaints when it is done.
pub fn spawn_check(state: UpdateRef, ctx: egui::Context) {
    *state.lock().unwrap() = UpdateState::Checking;
    std::thread::spawn(move || {
        let next = match check() {
            Ok(r) if update_available(env!("VF_COMMIT"), &r.commit) => UpdateState::Available(r),
            Ok(_) => UpdateState::UpToDate,
            Err(e) => UpdateState::Failed(e),
        };
        *state.lock().unwrap() = next;
        ctx.request_repaint();
    });
}

/// What to relaunch once installed.
enum Installed {
    App(PathBuf),
    Binary(PathBuf),
}

/// Download and install in a thread (the .dmg from an app, the binary otherwise), then
/// relaunch once no dictation or mini-session is running (one may have started meanwhile).
pub fn spawn_install(state: UpdateRef, release: Release, shared: SharedRef, ctx: egui::Context) {
    *state.lock().unwrap() = UpdateState::Downloading;
    std::thread::spawn(move || {
        let installed = match running_bundle() {
            Some(bundle) => install_dmg(&release, &bundle).map(|()| Installed::App(bundle)),
            None => install(&release).map(Installed::Binary),
        };
        let installed = match installed {
            Ok(i) => i,
            Err(e) => {
                *state.lock().unwrap() = UpdateState::Failed(e);
                ctx.request_repaint();
                return;
            }
        };
        loop {
            let busy = {
                let sh = shared.lock().unwrap();
                install_blocked(sh.phase, sh.sessions.iter().filter(|s| s.state == SessionState::Running).count()).is_some()
            };
            if !busy {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let relaunched = match &installed {
            Installed::App(bundle) => relaunch_app(bundle),
            Installed::Binary(path) => relaunch(path),
        };
        if let Err(e) = relaunched {
            *state.lock().unwrap() = UpdateState::Failed(e);
            ctx.request_repaint();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_latest_json() {
        let r = parse_release(r#"{"version":"0.3.0","commit":"F0A67A9","sha256":"ab12"}"#).unwrap();
        assert_eq!(r, Release { version: "0.3.0".into(), commit: "F0A67A9".into(), sha256: "ab12".into(), dmg_sha256: None });
    }

    #[test]
    fn parses_latest_json_with_dmg() {
        let r = parse_release(r#"{"version":"0.4.0","commit":"c0ffee","sha256":"ab12","dmg_sha256":"cd34"}"#).unwrap();
        assert_eq!(r.sha256, "ab12");
        assert_eq!(r.dmg_sha256.as_deref(), Some("cd34"));
        let empty = parse_release(r#"{"version":"0.4.0","commit":"c0ffee","sha256":"ab12","dmg_sha256":""}"#).unwrap();
        assert_eq!(empty.dmg_sha256, None);
    }

    #[test]
    fn bundle_from_exe_path() {
        let exe = Path::new("/Applications/Voix Fluide.app/Contents/MacOS/voix-fluide");
        assert_eq!(bundle_of(exe), Some(PathBuf::from("/Applications/Voix Fluide.app")));
        let support = Path::new("/Users/a/Library/Application Support/VoixFluide/programme/voix-fluide");
        assert_eq!(bundle_of(support), None);
        assert_eq!(bundle_of(Path::new("/Users/a/dev/voix-fluide/target/debug/voix-fluide")), None);
        assert_eq!(bundle_of(Path::new("/tmp/Voix Fluide/Contents/MacOS/voix-fluide")), None);
    }

    #[test]
    fn refuses_translocated_or_mounted_app() {
        let moved = Path::new("/private/var/folders/xy/T/AppTranslocation/1234-ABCD/d/Voix Fluide.app");
        assert!(bundle_place_refused(moved).unwrap().contains("copie temporaire"));
        assert!(bundle_place_refused(Path::new("/Volumes/Voix Fluide/Voix Fluide.app")).unwrap().contains("image disque"));
        assert_eq!(bundle_place_refused(Path::new("/Applications/Voix Fluide.app")), None);
        assert_eq!(bundle_place_refused(Path::new("/Volumes/Data/Apps/Voix Fluide.app")), None);
    }

    #[test]
    fn rejects_missing_field_and_bad_json() {
        let e = parse_release(r#"{"version":"0.3.0","commit":"f0a67a9"}"#).unwrap_err();
        assert!(e.contains("sha256"), "{e}");
        assert!(parse_release(r#"{"version":"0.3.0","commit":"","sha256":"ab"}"#).unwrap_err().contains("commit"));
        assert!(parse_release("<html>Not Found</html>").is_err());
    }

    #[test]
    fn update_when_commit_differs() {
        assert!(!update_available("f0a67a9c", "F0A67A9C"));
        assert!(update_available("f0a67a9c", "275a6b4d"));
        assert!(update_available("unknown", "unknown"));
    }

    #[test]
    fn checks_at_launch_only_for_release_builds() {
        assert!(check_at_launch(true, false));
        assert!(!check_at_launch(false, false));
        assert!(!check_at_launch(true, true));
    }

    #[test]
    fn install_waits_for_dictation_and_sessions() {
        assert_eq!(install_blocked(Phase::Idle, 0), None);
        assert!(install_blocked(Phase::Recording, 0).unwrap().contains("dictée"));
        assert!(install_blocked(Phase::Transcribing, 0).unwrap().contains("dictée"));
        assert!(install_blocked(Phase::Idle, 1).unwrap().contains("mini-session"));
    }

    #[test]
    fn sha256_of_a_small_file() {
        let dir = std::env::temp_dir().join(format!("vf-sha-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("abc.txt");
        std::fs::write(&f, b"abc").unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(sha256_of(&f).unwrap(), abc);
        assert!(sha256_matches(&f, &abc.to_uppercase()).unwrap());
        assert!(!sha256_matches(&f, "00").unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
