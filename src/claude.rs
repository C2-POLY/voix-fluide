//! Mini-sessions: a spoken question sent to Claude Code (`claude -p`) in one of the
//! person's repositories, read-only. Not in the original app (added in this rebuild).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Read, search and list files only: no writing, no shell.
pub const TOOLS: &str = "Read,Grep,Glob";

const SYSTEM: &str = "La question a été posée à voix haute sur ce dépôt ; tu ne peux que lire le code, sans rien modifier. Réponds en français. Dans « resume », une seule phrase courte qui sera lue à voix haute ; dans « reponse », la réponse complète.";

/// What a finished mini-session gives back.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub session_id: String,
    pub resume: String,
    pub reponse: String,
}

/// Where the Claude Code command line usually lives. An app opened from the Finder
/// has a short PATH, so the usual install places are checked too.
pub fn find_claude() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).map(|d| d.join("claude")).collect())
        .unwrap_or_default();
    candidates.push(home.join(".local/bin/claude"));
    candidates.push(home.join(".claude/local/claude"));
    candidates.push(PathBuf::from("/opt/homebrew/bin/claude"));
    candidates.push(PathBuf::from("/usr/local/bin/claude"));
    candidates.into_iter().find(|p| p.is_file())
}

/// `~/…` to the home folder.
pub fn expand_home(path: &str) -> PathBuf {
    match path.trim().strip_prefix("~/") {
        Some(rest) => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest),
        None => PathBuf::from(path.trim()),
    }
}

/// Lower case, accents dropped, anything but letters and digits as one space,
/// so "Voix-Fluide" and "voix fluide" both give "voix fluide".
fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        };
        if c != ' ' || !out.ends_with(' ') {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// The repository the sentence names among the folders of `root` (whole words, case
/// and accents ignored, `-` and `_` read as spaces, hidden folders skipped, the longest
/// name wins); otherwise `last`; otherwise None.
pub fn route(text: &str, root: &Path, last: Option<&Path>) -> Option<PathBuf> {
    let said = format!(" {} ", fold(text));
    let named = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let key = fold(&name);
            (!name.starts_with('.') && !key.is_empty() && said.contains(&format!(" {key} ")))
                .then(|| (key.len(), e.path()))
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, p)| repo_dir(p));
    named.or_else(|| last.map(Path::to_path_buf))
}

/// A folder that is not a Git repository but holds exactly one (a workspace with
/// its code in `app/`, say) stands for that repository.
fn repo_dir(folder: PathBuf) -> PathBuf {
    if folder.join(".git").exists() {
        return folder;
    }
    let mut inner = std::fs::read_dir(&folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join(".git").exists());
    match (inner.next(), inner.next()) {
        (Some(only), None) => only,
        _ => folder,
    }
}

/// The answer's shape: one spoken sentence and the full answer.
pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "resume": {"type": "string", "description": "Une seule phrase courte, lue à voix haute."},
            "reponse": {"type": "string", "description": "La réponse complète."}
        },
        "required": ["resume", "reponse"],
        "additionalProperties": false
    })
}

/// The arguments of `claude` for one question: print mode, read-only tools, JSON out,
/// only the project's settings. The session is kept on disk so `--resume` works.
pub fn args(prompt: &str) -> Vec<String> {
    [
        "-p", prompt,
        "--tools", TOOLS,
        "--output-format", "json",
        "--setting-sources", "project",
        "--json-schema", &schema().to_string(),
        "--append-system-prompt", SYSTEM,
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Run the question in `repo` and wait for the answer. Blocks: call it from a thread.
pub fn run(prompt: &str, repo: &Path) -> Result<Answer, String> {
    let exe = find_claude().ok_or(
        "Claude Code introuvable : installez-le (claude.com/claude-code), puis lancez `claude` une fois dans le Terminal.",
    )?;
    let out = Command::new(exe)
        .args(args(prompt))
        .current_dir(repo)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Claude Code n'a pas pu démarrer : {e}"))?;
    parse(&String::from_utf8_lossy(&out.stdout), &String::from_utf8_lossy(&out.stderr))
}

/// Read `claude -p --output-format json`'s reply.
pub fn parse(stdout: &str, stderr: &str) -> Result<Answer, String> {
    let short = |s: &str| s.trim().chars().take(300).collect::<String>();
    // Some errors print a line before the JSON: the last JSON line counts too.
    let reply = serde_json::from_str::<Value>(stdout.trim())
        .ok()
        .or_else(|| stdout.lines().rev().find_map(|l| serde_json::from_str::<Value>(l.trim()).ok()))
        .filter(Value::is_object);
    let Some(v) = reply else {
        let detail = if stderr.trim().is_empty() { short(stdout) } else { short(stderr) };
        return Err(format!("Réponse illisible de Claude Code : {detail}"));
    };
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    if v.get("is_error").and_then(Value::as_bool).unwrap_or(false) {
        let why = Some(text(&v, "result")).filter(|s| !s.is_empty()).unwrap_or_else(|| text(&v, "subtype"));
        return Err(format!("Claude n'a pas pu répondre : {}", short(&why)));
    }
    let session_id = text(&v, "session_id");
    let structured = v.get("structured_output").filter(|s| s.is_object()).cloned().or_else(|| {
        serde_json::from_str::<Value>(&text(&v, "result")).ok().filter(Value::is_object)
    });
    match structured {
        Some(s) if !text(&s, "reponse").is_empty() => Ok(Answer {
            session_id,
            resume: text(&s, "resume"),
            reponse: text(&s, "reponse"),
        }),
        _ => Err(format!("Réponse illisible de Claude Code : {}", short(stdout))),
    }
}

/// The command that continues a mini-session in a terminal.
pub fn resume_command(repo: &Path, session_id: &str) -> String {
    let dir = repo.to_string_lossy();
    let dir = if dir.chars().all(|c| c.is_ascii_alphanumeric() || "/._-~".contains(c)) {
        dir.to_string()
    } else {
        format!("'{}'", dir.replace('\'', r"'\''"))
    };
    format!("cd {dir} && claude --resume {session_id}")
}

/// The script Terminal runs to continue a mini-session: a login shell (the person's
/// PATH), then `claude --resume` in the repository.
pub fn resume_script(repo: &Path, session_id: &str) -> String {
    format!("#!/bin/zsh -l\n{}\n", resume_command(repo, session_id))
}

/// Open a Terminal window that continues the mini-session. A `.command` file opened
/// with `open` runs in Terminal without asking for the Automation permission.
pub fn open_in_terminal(repo: &Path, session_id: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = crate::whisper::app_dir().join("sessions");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("reprendre-{session_id}.command"));
    std::fs::write(&file, resume_script(repo, session_id)).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    let status = Command::new("open").arg(&file).status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`open` a échoué ({status})"))
    }
}

/// Read a sentence aloud with macOS's `say`, without waiting.
pub fn say(text: &str) {
    if !text.trim().is_empty() {
        let _ = Command::new("say").arg(text).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vf-route-{}", std::process::id()));
        for d in ["fourchette", "voix-fluide", "voix-fluide-mini-session", "Éclair_App", ".cache", "app"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        std::fs::write(dir.join("notes"), "").unwrap();
        dir
    }

    #[test]
    fn route_names_a_repository() {
        let r = root();
        assert_eq!(route("Dans Fourchette, quelle est la branche principale ?", &r, None), Some(r.join("fourchette")));
        assert_eq!(route("dans voix fluide, où est la pastille ?", &r, None), Some(r.join("voix-fluide")));
        assert_eq!(route("Dans VOIX FLUIDE MINI SESSION, quoi de neuf ?", &r, None), Some(r.join("voix-fluide-mini-session")));
        assert_eq!(route("Dans éclair app, que fait main ?", &r, None), Some(r.join("Éclair_App")));
        assert_eq!(route("Dans eclair-app ?", &r, None), Some(r.join("Éclair_App")));
    }

    #[test]
    fn route_enters_the_one_repository_of_a_workspace() {
        let r = std::env::temp_dir().join(format!("vf-workspace-{}", std::process::id()));
        std::fs::create_dir_all(r.join("voix-fluide/app/.git")).unwrap();
        std::fs::create_dir_all(r.join("voix-fluide/checks")).unwrap();
        std::fs::create_dir_all(r.join("fourchette/.git")).unwrap();
        std::fs::create_dir_all(r.join("fourchette/app/.git")).unwrap();
        std::fs::create_dir_all(r.join("deux/a/.git")).unwrap();
        std::fs::create_dir_all(r.join("deux/b/.git")).unwrap();
        assert_eq!(route("Dans voix fluide ?", &r, None), Some(r.join("voix-fluide/app")));
        assert_eq!(route("Dans fourchette ?", &r, None), Some(r.join("fourchette")));
        assert_eq!(route("Dans deux ?", &r, None), Some(r.join("deux")));
    }

    #[test]
    fn route_skips_hidden_folders_files_and_partial_words() {
        let r = root();
        assert_eq!(route("dans cache, quoi ?", &r, None), None);
        assert_eq!(route("lis les notes", &r, None), None);
        assert_eq!(route("une application", &r, None), None);
    }

    #[test]
    fn route_falls_back_to_the_last_repository() {
        let r = root();
        let last = r.join("fourchette");
        assert_eq!(route("Et les tests, ils passent ?", &r, Some(&last)), Some(last.clone()));
        assert_eq!(route("Dans voix fluide, et les tests ?", &r, Some(&last)), Some(r.join("voix-fluide")));
        assert_eq!(route("Et les tests ?", &r, None), None);
        assert_eq!(route("Dans fourchette ?", Path::new("/nonexistent-vf"), None), None);
    }

    #[test]
    fn args_are_read_only_and_keep_the_session() {
        let a = args("Que fait ce dépôt ?");
        assert_eq!(&a[..2], ["-p", "Que fait ce dépôt ?"]);
        let after = |flag: &str| a[a.iter().position(|x| x == flag).unwrap() + 1].clone();
        assert_eq!(after("--tools"), "Read,Grep,Glob");
        assert_eq!(after("--output-format"), "json");
        assert_eq!(after("--setting-sources"), "project");
        let schema: Value = serde_json::from_str(&after("--json-schema")).unwrap();
        assert_eq!(schema["required"], json!(["resume", "reponse"]));
        assert!(after("--append-system-prompt").contains("français"));
        assert!(!a.iter().any(|x| x == "--no-session-persistence"));
    }

    #[test]
    fn parse_reads_a_structured_answer() {
        let out = r#"{"type":"result","subtype":"success","is_error":false,"session_id":"abc-123","result":"","structured_output":{"resume":"La branche principale est main.","reponse":"La branche principale est main, protégée."}}"#;
        assert_eq!(
            parse(out, ""),
            Ok(Answer {
                session_id: "abc-123".into(),
                resume: "La branche principale est main.".into(),
                reponse: "La branche principale est main, protégée.".into(),
            })
        );
        // A warning line before the JSON.
        let noisy = format!("[warn] something\n{out}\n");
        assert_eq!(parse(&noisy, "").unwrap().session_id, "abc-123");
    }

    #[test]
    fn parse_reports_errors() {
        let out = r#"{"type":"result","subtype":"error_max_turns","is_error":true,"session_id":"x","result":"Credit balance is too low"}"#;
        assert_eq!(parse(out, "").unwrap_err(), "Claude n'a pas pu répondre : Credit balance is too low");
        let out = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"session_id":"x"}"#;
        assert!(parse(out, "").unwrap_err().ends_with("error_during_execution"));
        assert!(parse("not json at all", "boom").unwrap_err().starts_with("Réponse illisible de Claude Code : boom"));
        assert!(parse("", "").is_err());
        assert!(parse(r#"{"type":"result","is_error":false,"session_id":"x","result":"hello"}"#, "").is_err());
    }

    #[test]
    fn resume_command_quotes_odd_paths() {
        assert_eq!(resume_command(Path::new("/Users/me/repo/fourchette"), "id1"), "cd /Users/me/repo/fourchette && claude --resume id1");
        assert_eq!(resume_command(Path::new("/Users/me/Mes dépôts/x"), "id2"), "cd '/Users/me/Mes dépôts/x' && claude --resume id2");
    }

    #[test]
    fn resume_script_runs_in_a_login_shell() {
        assert_eq!(resume_script(Path::new("/r/fourchette"), "id1"), "#!/bin/zsh -l\ncd /r/fourchette && claude --resume id1\n");
    }

    /// Calls Claude Code for real (costs a request): `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn live_mini_session() {
        let dir = std::env::temp_dir().join(format!("vf-live-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("note.txt"), "Le mot de passe du jour est tournesol.\n").unwrap();
        let a = run("Que dit le fichier note.txt ?", &dir).expect("claude -p");
        println!("{a:?}");
        assert!(!a.session_id.is_empty());
        assert!(a.reponse.to_lowercase().contains("tournesol"));
    }
}
