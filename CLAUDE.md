# Voix Fluide

A dictation app for macOS in Rust and egui: hold or tap a key, speak, and the text is pasted into the active app. Speech to text only, entirely on the Mac (Whisper through candle on Metal, or NVIDIA Parakeet through ONNX Runtime). It began as a simpler rebuild of another dictation app.

## Project state

- This repository is the `app/` folder of the rebuild workspace. The workspace around it (`../investigation.cany/`, `../craft.json`, `../checks/`, `../agent-session.jsonl`…) is what the rebuild tool found and wrote: read it, never edit it by hand, never commit it.
- `MECHANICS.md` and `mechanics.json` hold the values measured in the original app, with their source (function and address). Keep them in sync when one of those values changes in the code.
- Git ignores `target/`, `.testhome/` (a fake home for tests) and `shots/`.

## Stack

Rust 2021, pinned to 1.85.1 by `rust-toolchain.toml` (rustup), eframe and egui 0.31 on glow, `cpal` for the microphone, `candle` 0.9.1 with Metal for Whisper, `parakeet-rs` and `ort` for Parakeet, `ureq` for the model downloads (Hugging Face), `arboard` for the clipboard. macOS APIs (accessibility, microphone permission, key state, paste) are called directly in `mac.rs`.

## Commands

Run them from the repository root (or a worktree's root), never with `--manifest-path` from another folder. rustup picks the toolchain from the current folder's `rust-toolchain.toml`. From the workspace folder above, it uses the default stable instead of 1.85.1, and every switch between the two recompiles all 324 crates (about 3 minutes) in the shared `target/`.

```bash
# Build and run
cargo run

# Unit tests (fast, offline)
cargo test

# Live and slow tests: they download a model and transcribe for real
cargo test -- --ignored --nocapture

# Lints: no warning at all, so any new one stands out
cargo clippy

# Transcribe a 16-bit WAV from the command line, with the model chosen in the settings (downloaded if missing)
cargo run -- --transcribe <file.wav> [language]

# Screenshot of the window (run from a folder such as /tmp)
(cd /tmp && <repo>/target/debug/voix-fluide --screenshot <out.png>)

# Screenshot of the floating pill in each of its states (keep --screenshot: it skips the permission relaunch)
(cd /tmp && VF_PILL_SHOT=<out.png> <repo>/target/debug/voix-fluide --screenshot /dev/null)
```

Other switches: `VF_PILL_DEMO=1` pretends to dictate between 6 and 10 seconds after launch, `VF_DEBUG=1` prints Whisper's decoding steps, `VF_SESSIONS=1` fills the Mini-sessions list with three sample sessions (running, done, failed) for `--screenshot`, `VF_TOAST=<text>` shows that toast at the bottom of the window. Both pictures are 1120x1440 pixels (560x720 points on a Retina screen).

## Architecture

- `main.rs`: arguments (`--screenshot`, `--transcribe`, `--signature-check`), the window (`App`: settings, history, permissions) and the floating pill (`draw_pill`, `pill_layout`).
- `engine.rs`: the settings (`Settings`, kept in `settings.json`), the shared state (`Shared` behind a `Mutex`, with the mini-sessions and `last_repo`), and two threads: `controller` (the two shortcut keys, tap or hold, the microphone through `cpal`, `Cmd`s carrying a `Target`) and `worker` (loading the model and transcribing, `Job`s; a `Target::Session` text starts a mini-session in its own thread instead of being pasted). Silence and dead-microphone checks, resampling to 16 kHz, the history (50 entries).
- `claude.rs`: mini-sessions. Finds the `claude` binary (PATH, then the usual install places), picks the repository named in the sentence (`route`), builds the read-only `claude -p` arguments (`args`), runs it in the repository and reads its JSON (`run`, `parse`).
- `whisper.rs`: the models (`ModelSize`, Whisper and Parakeet), their download into `~/Library/Application Support/VoixFluide/models/`, and the Whisper decoder on candle.
- `mac.rs`: macOS. The shortcut key state, the accessibility and microphone permissions, the relaunch that answers for its own permissions (`run_with_own_permissions`, a signed copy in `VoixFluide/programme/`), and the paste into the active app (`paste_text`, a Cmd+V posted after filling the clipboard, which it can put back).
- `build.rs`: embeds an Info.plist (bundle id, name, version, microphone and accessibility reasons); without it macOS gives the microphone as silence.

Everything the app writes lives in `~/Library/Application Support/VoixFluide/` (`settings.json`, `models/`, `programme/`).

## Site

`site/` is the presentation page, published at <https://c2-poly.github.io/voix-fluide/> by `.github/workflows/pages.yml` on every merge into `main` that touches it (a pull request only builds it). Vite, React, TypeScript, Tailwind 4, GSAP (ScrollTrigger, SplitText, DrawSVG) and Lenis; `npm` from inside `site/`: `npm install`, `npm run dev` (served under `/voix-fluide/`), `npm run build` (typecheck and build).

- Its text lives in `src/i18n.ts`, in French and English: change both together, and keep every figure true to `MECHANICS.md` and the code.
- Look: "paper and ink" (paper `#f2ece1`, ink `#16130f`, one vermilion accent, Fraunces and IBM Plex Mono). No purple gradient, glass effect or particle background.
- `public/fenetre.png` is the window screenshot (`VF_SESSIONS=1 … --screenshot`); retake it when the window changes. The pill is redrawn in HTML (`components/Pill.tsx`), not a picture.
- Every animation goes through `gsap.matchMedia()` or checks `reducedMotion()`, so the page stays still with "Reduce motion".

## Work items (GitHub)

Work items are GitHub issues of `C2-POLY/voix-fluide`, tracked in the organisation's project **Voix Fluide** (<https://github.com/orgs/C2-POLY/projects/5>, number 5). `gh` must be signed in with the `project` scope (`gh auth refresh -s project`).

Issues are written in French and kept short:

- Title: `<Zone> : <ce qu'il apporte>`, for example `Pastille : afficher la langue détectée`.
- Body: `**En tant que** …, **je veux** …, **afin de** …`, then `**Critères d'acceptation**` with 3 to 5 checkboxes, then `**Livré** : \`vX.Y.Z\`` once delivered.
- Type: the organisation's issue types, `Feature` (a capability, a parent issue when it is split), `Task` (an improvement, tooling, docs) or `Bug`. Split work is sub-issues; "B cannot start before A" is a `blocked by` relation, not a sentence.
- Labels: one `zone:` label (`zone:raccourci`, `zone:micro`, `zone:transcription`, `zone:modèles`, `zone:pastille`, `zone:collage`, `zone:fenêtre`, `zone:réglages`, `zone:permissions`, `zone:claude`, `zone:outillage`), and once delivered one version label (`v0.2.0`). Create a missing label rather than skipping it.
- Assignee: the person who did the work.
- Status on the project: `Backlog` (defined), `Ready` (ready to take), `In progress` (its worktree exists), `In review` (PR open), `To test` (merged, a criterion still untested by hand), `Done` (tested by the dev and merged).
- Every delivered change has an issue, including small ones only visible in `git log`.
- The PR closes its issue (`Closes #12` in its body).

Project ids for `gh project item-edit`: project `PVT_kwDODd-WqM4Bq3jr`, Status field `PVTSSF_lADODd-WqM4Bq3jrzhpMuYM`, options Backlog `437035ff`, Ready `2f064209`, In progress `4ac2b093`, In review `c87c94e2`, To test `92b34e20`, Done `eb01cdc3`.

## Versioning

- The version is `version` in `Cargo.toml`, as `MAJOR.MINOR.PATCH`, and `CFBundleShortVersionString` in `build.rs` must say the same.
- One working day is one minor version (0.x while the app is not distributed). Bump it in the first commit of the day that changes something visible in the app, inside that commit. A day that only fixes bugs bumps the patch instead. Commits that only touch docs or tooling do not bump it.
- Never put the version number in a commit message.
- After each merge into `main`, move the current version's tag to `main`. Tags only ever point at `main`:

```bash
git fetch --tags
git tag -f v0.1.0 origin/main && git push -f origin v0.1.0
```

## Commits and language

Commit messages, PR titles and code are in English and follow Conventional Commits (`feat(pill): …`, `fix(paste): …`). Issues and talk with the dev are in French. No AI attribution in commits, PRs or issues.
