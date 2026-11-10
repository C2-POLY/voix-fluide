//! Updates: read the latest GitHub Release, download its binary, check it,
//! install it in `programme/source/` and relaunch from it.

use crate::engine::{Phase, SessionState, SharedRef};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const LATEST_JSON: &str = "https://github.com/C2-POLY/voix-fluide/releases/latest/download/latest.json";
const LATEST_BINARY: &str = "https://github.com/C2-POLY/voix-fluide/releases/latest/download/voix-fluide-aarch64-apple-darwin";

/// What the Updates page shows.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available { version: String, commit: String, sha256: String },
    Downloading,
    Failed(String),
}

pub type UpdateRef = Arc<Mutex<UpdateState>>;

/// The fields of the Release's `latest.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub commit: String,
    pub sha256: String,
}

/// Read `latest.json`: `{"version", "commit", "sha256"}`, all non-empty strings.
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
    Ok(Release { version: field("version")?, commit: field("commit")?, sha256: field("sha256")? })
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
        "available" => UpdateState::Available {
            version: env!("CARGO_PKG_VERSION").into(),
            commit: "abc1234def5678901234567890abcdef12345678".into(),
            sha256: String::new(),
        },
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
    // 34 MB: a connection timeout, then a read timeout instead of a total one.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .build();
    let resp = agent.get(LATEST_BINARY).call().map_err(http_error)?;
    let mut out = std::fs::File::create(part).map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = reader.read(&mut buf).map_err(|_| "Téléchargement interrompu.".to_string())?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    if !sha256_matches(part, sha256)? {
        return Err("Le fichier téléchargé ne correspond pas à la Release (SHA-256).".into());
    }
    std::fs::set_permissions(part, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    if !starts(part) {
        return Err("Le programme téléchargé ne démarre pas.".into());
    }
    Ok(())
}

/// Start the installed program as a fresh launcher (without the copy's marker, so it
/// goes through `run_with_own_permissions` and re-signs), detached, then quit.
pub fn relaunch(path: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let mut cmd = std::process::Command::new(path);
    cmd.env_remove(crate::mac::CHILD_ENV)
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
            Ok(r) if update_available(env!("VF_COMMIT"), &r.commit) => {
                UpdateState::Available { version: r.version, commit: r.commit, sha256: r.sha256 }
            }
            Ok(_) => UpdateState::UpToDate,
            Err(e) => UpdateState::Failed(e),
        };
        *state.lock().unwrap() = next;
        ctx.request_repaint();
    });
}

/// Download and install in a thread, then relaunch once no dictation or
/// mini-session is running (one may have started during the download).
pub fn spawn_install(state: UpdateRef, release: Release, shared: SharedRef, ctx: egui::Context) {
    *state.lock().unwrap() = UpdateState::Downloading;
    std::thread::spawn(move || {
        let path = match install(&release) {
            Ok(p) => p,
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
        if let Err(e) = relaunch(&path) {
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
        assert_eq!(r, Release { version: "0.3.0".into(), commit: "F0A67A9".into(), sha256: "ab12".into() });
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
