# Compliance findings

- Repo: `C2-POLY/voix-fluide`
- Scanned commit: `3e7e03c8c879a1a1429eef54d3ad13cbbacb4865`
- Scanned on: `2026-11-11`
- Scope: the whole repo: the macOS app (`src/`, `build.rs`) and the presentation site (`site/`)

This file lists what the code shows. It prepares documentation; it does not certify compliance.

## personal_data

| # | What | Where (file:line) | Confidence | Becomes |
| --- | --- | --- | --- | --- |
| P1 | Microphone recording of each dictation (16 kHz mono PCM), sent to the local speech-to-text model; the last one is written as a WAV file so it can be played back | `src/engine.rs:650-671`, `src/engine.rs:728` | found | personal_data/voice-recording, question 3 |
| P2 | Text transcribed from the dictation, pasted into the active app and kept in a 50-entry history shown in the window; its content is whatever the person says | `src/engine.rs:866-931`, `src/engine.rs:355-360` | found | personal_data/dictated-text, question 3 |
| P3 | Spoken question to Claude Code (transcribed text), the repository it is asked in (a folder path, which holds the macOS account name), the answer and the Claude session id | `src/claude.rs:139-147`, `src/engine.rs:338-347`, `src/engine.rs:948-993` | found | personal_data/claude-questions, question 3 |
| P4 | The person's Apple code-signing certificates read from the keychain (`security find-identity`, `security find-certificate`); a certificate names its holder. The hash of the identity that worked is remembered | `src/mac.rs:340-393`, `src/mac.rs:400-445` | found | personal_data/signing-certificate, question 3 |

## storage

| # | What | Where (file:line) | Confidence | Becomes |
| --- | --- | --- | --- | --- |
| S1 | Last recording written to `~/Library/Application Support/VoixFluide/derniere-dictee.wav`, overwritten by the next recording, never deleted by the app | `src/engine.rs:650-671`, `src/whisper.rs:102-105` | found | storage/last-recording |
| S2 | Dictation history (50 entries, text, time, length) and the mini-sessions (question, repository, answer, session id) held in process memory only; nothing writes them to disk | `src/engine.rs:362-381`, `src/engine.rs:912-920` | found | storage/in-app-memory |
| S3 | The system clipboard receives the dictated text for the paste; the previous clipboard text is put back unless "keep the clipboard" is on or something else replaced it | `src/mac.rs:565-602`, `src/mac.rs:603` | found | storage/clipboard |
| S4 | `settings.json` in the app folder holds the repositories folder (`repo_root`, a path) among the settings; rewritten on change, never deleted by the app | `src/engine.rs:196-305` | found | storage/settings-file |
| S5 | One `sessions/reprendre-<session id>.command` script per mini-session continued in Terminal, holding the repository path and the session id; never deleted by the app | `src/claude.rs:197-212` | found | storage/session-scripts |
| S6 | `programme/identity.txt` keeps the hash of the signing identity used; `programme/cert.pem` holds one certificate only while it is checked, then is deleted | `src/mac.rs:350-393`, `src/mac.rs:466-477` | found | storage/signing-identity |

## processors

None found in the code. No provider key, token or contract of the organisation appears in the app: every recipient is reached anonymously or with the person's own tool. GitHub, as host of the site (O5), is recorded as a processor from the answer to question 5.

## outbound_flows

| # | What | Where (file:line) | Confidence | Becomes |
| --- | --- | --- | --- | --- |
| O1 | `claude -p <question>` run in the chosen repository with read-only tools (`Read,Grep,Glob`); Claude Code sends the question and the files it reads to its provider. The account is the one the person signed in to Claude Code with (inferred: the app passes no key and runs the person's own binary) | `src/claude.rs:118-147`, `src/claude.rs:22-33` | found | outbound_flows/ask-claude-code, question 4, question 5 |
| O2 | Update check: `latest.json` of the latest GitHub Release, then the binary or `.dmg` from the same Release; no identifier sent, only the IP address and technical headers | `src/update.rs:12-14`, `src/update.rs:108-113`, `src/update.rs:198-215` | found | outbound_flows/update-check, question 4 |
| O3 | Model download from Hugging Face (Whisper and Parakeet files); no identifier sent, only the IP address and technical headers | `src/whisper.rs:88-97`, `src/whisper.rs:121-140` | found | outbound_flows/model-download, question 4 |
| O4 | Revocation check of a signing certificate with Apple (`security verify-cert … -R ocsp`): the OCSP request names the certificate | `src/mac.rs:466-485` | found | outbound_flows/certificate-check, question 4 |
| O5 | The presentation site is published on GitHub Pages (`c2-poly.github.io/voix-fluide`); the host sees each visitor's IP address and technical headers. The site loads no tracker, no analytics and no external font (fonts are bundled from `@fontsource`) | `site/vite.config.ts:5`, `site/index.html`, `site/src/index.css:1-5` | inferred | outbound_flows/site-hosting, processors/github-pages, question 4, question 5 |

## automated_decisions

None found.

## ai_features

| # | What | Where (file:line) | Confidence | Becomes |
| --- | --- | --- | --- | --- |
| A1 | Speech-to-text on the Mac (Whisper on candle, or Parakeet on ONNX Runtime): generates text from the voice. The text is pasted into the active app with no step in between (`human_review: false`, found) | `src/whisper.rs`, `src/engine.rs:866-931` | found | ai_features/dictation-transcription, question 6 |
| A2 | Mini-sessions: Claude Code answers a spoken question about a repository; the answer is shown in the window and its one-sentence summary is read aloud with `say`. Nothing is applied: Claude has read-only tools and the person reads the answer (`human_review: true`, inferred). The page and the shortcut are labelled "Claude Code" | `src/claude.rs:8-147`, `src/engine.rs:948-993`, `src/main.rs:1176`, `src/main.rs:1306` | found | ai_features/claude-mini-sessions |

## Not counted

| What | Where | Why |
| --- | --- | --- |
| Test modules (`#[cfg(test)]`), including a fake note with a made-up password in `claude.rs` | `src/claude.rs:221-340`, `src/engine.rs:1049-`, `src/mac.rs:609-`, `src/update.rs:453-` | Tests, not shipped |
| `--transcribe <file.wav>` prints the text on standard output | `src/main.rs:60-74` | A command-line tool for the developer; the text goes to the person's own terminal, nothing is stored |
| `VF_DEBUG` prints decoding scores | `src/whisper.rs:483` | Scores only, no text |
| Claude Code keeps its own session transcripts on the Mac | (outside this repo) | Storage of a third-party tool the person installed, not of this app |
| The paste into the active app and `say` reading the summary aloud | `src/mac.rs:565`, `src/claude.rs:214-219` | Local to the Mac, no recipient |
| `build.rs` runs `git rev-parse HEAD` | `build.rs:35` | Build time only |
| Release and site workflows, bundling scripts | `.github/workflows/`, `scripts/` | Dev-only tooling |

Dev-only files that may hold personal data: none found.

## Questions

1. Who is the controller: the organisation, the user of the app, or shared? (feeds service.controller)
2. Who is answering? (source of the answers)
3. Legal basis of voice-recording, dictated-text, claude-questions and signing-certificate (feeds personal_data/*.legal_basis)
4. Country of Anthropic (through Claude Code), GitHub (Releases and Pages), Hugging Face and Apple as recipients (feeds outbound_flows/*.to.country)
5. Whose account: ask-claude-code (inferred user), site-hosting (inferred organisation); for GitHub Pages, whether the organisation has a data processing agreement with GitHub (feeds outbound_flows/*.account, may turn site-hosting into a processor)
6. How is the person told that dictation-transcription text is produced by a model? (feeds ai_features/dictation-transcription.marking)
7. Which repository is the organisation's compliance repo? (feeds .github/workflows/compliance.yml)
