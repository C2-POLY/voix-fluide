# Voix Fluide — mechanics sheet

Values read from FluidVoice 1.6.12 (`/Applications/FluidVoice.app/Contents/MacOS/FluidVoice`, arm64 Mach-O, Swift).
Addresses are in that binary. "estimated" means it was looked for and not found.

## Dictation shortcut

| Rule | Value | Source |
|---|---|---|
| Default dictation key | keyCode `0x3D` (61) = **Right Option**, no modifiers | `SettingsStore.primaryDictationShortcuts` getter, 0x1000bc388 — `HotkeyShortcut(keyCode: 0x3d, modifierFlags: 0)` when `HotkeyShortcutKey` is unset |
| Activation modes | `toggle` = "Toggle", `hold` = "Hold", `automatic` = "Automatic (Both)" | `HotkeyActivationMode.displayName` 0x10009f1cc |
| Mode help texts | "Tap once to start, tap again to stop." / "Record only while the shortcut is held." / "Tap to toggle, hold for push-to-talk." | `HotkeyActivationMode.description` 0x10009f21c |
| Default mode | Toggle (legacy `PressAndHoldMode` bool, false when unset) | `SettingsStore.hotkeyMode` 0x1000bd290 — fallback read inferred |
| Tap/hold threshold (Automatic) | **0.4 s** (`0x3FD999999999999A`) — press shorter = toggle, longer = push-to-talk | `GlobalHotkeyManager.automaticTapThresholdSeconds` initializer 0x10069f624 |

## Audio and transcription

| Rule | Value | Source |
|---|---|---|
| Engine kept for this version | Whisper (the original also ships Parakeet/Apple/Cohere; Whisper is the one that "works on any Mac") | strings "Whisper models support 99 languages and work on any Mac." |
| Sample rate | 16 000 Hz mono | whisper minimum check below is in 16 kHz samples |
| Minimum audio | `samples < 16000` (count >> 7 < 0x7D) → "Audio too short for Whisper transcription" = 1 s | `WhisperProvider.transcribe` 0x100802778 |
| Whisper sizes | Tiny (~75 MB), Base (~142 MB), Small (~466 MB), Medium (~1.5 GB), Large (~2.9 GB) | `WhisperModelSize.displayName` 0x1000e5840 |
| Default size | index 1 = **Base** | `SettingsStore.whisperModelSize` 0x1000e4210 (`w0 = 1` fallback) |
| Language | per-user setting, auto when unset | `WhisperProvider.transcribe` reads `selectedWhisperLanguageCode` |

## Microphone and silence

| Rule | Value | Source |
|---|---|---|
| Dead-microphone watchdog | a buffer counts as dead when RMS ≤ **1e-5** (0x3727C5AC) and peak < **5e-5** (0x3851B717); after 3 dead buffers in a row FluidVoice restarts capture | `AudioCaptureIdlePolicy.SilentPCMRecoveryWatchdog.maximumRMS` 0x10056f890, `.maximumPeak` 0x10056f8ac, `.shouldRecover` 0x10056f8bc |
| This rebuild | a whole recording under those two limits is not transcribed; the window explains that macOS is not giving the microphone and offers "Autoriser le micro" / "Ouvrir les réglages du micro" | — (cause measured on this Mac: every input gave peak 0.000000 while the microphone permission was "not asked") |
| Short-silence skip | 20 ms frames (sampleRate / 50); skip when overall RMS < **0.002** (0x3B03126F), peak < **0.01** (0x3C23D70A) and loudest frame RMS < **0.0045** (0x3B9374BC) | `ASRService.assessShortAudioSilence` 0x10054e294 |
| Short-silence limits | only recordings "up to four seconds" (4000 ms — estimated from the setting text); setting `SkipSilentRecordingsEnabled` off by default ("Disabled by default to preserve quiet speech") | strings, `SettingsStore.skipSilentRecordingsEnabled` 0x1000be4e8 |
| Microphone choice | `preferredMicrophoneUID` / "MicrophonePriority", falls back to the system default ("Trying system default input.") | strings |
| Bluetooth hint | "Bluetooth microphone mode can reduce headphone playback quality. Prefer a wired, USB, or display microphone when available." | string |
| Level meter silence threshold | 0.04 | `AudioCapturePipeline.silenceThreshold` initializer 0x100545074 |
| Whisper "no speech" rule (this rebuild) | drop a 30 s window when P(<\|nocaptions\|>) after <\|startoftranscript\|> > 0.6 and mean log-prob < −1 (Whisper's own constants), or when P > 0.85 alone — the 0.85 is **estimated** (measured: speech 0.000–0.006, silence 0.93, faint noise 0.91) | candle `NO_SPEECH_THRESHOLD`, `LOGPROB_THRESHOLD` |
| Resampling (this rebuild) | windowed-sinc (Blackman, 16 zero crossings, cut-off 0.92 × new Nyquist) to 16 kHz; FluidVoice uses AVAudioConverter — estimated equivalent | string "AVAudioConverterInputStatus" |
| Microphone permission | the program embeds an Info.plist with `NSMicrophoneUsageDescription` and restarts itself as its own "responsible" process, so macOS asks Voix Fluide (not the app that started it) | Info.plist of FluidVoice: `NSMicrophoneUsageDescription` |

## Floating pill (dictation overlay)

| Rule | Value | Source |
|---|---|---|
| Overlay positions | "Top of Screen" / "Bottom of Screen"; default **Bottom of Screen** (getter returns case 1 when `OverlayPosition` unset) | `SettingsStore.OverlayPosition.displayName` 0x1000c3d94, `SettingsStore.overlayPosition` 0x1000c434c |
| Distance from the screen edge | **50 pt** (`OverlayBottomOffset`, 0x4049000000000000 when 0/unset) | `SettingsStore.overlayBottomOffset` 0x1000c48cc |
| Overlay sizes | 0 "Pill", 1 "Small", 2 "Medium", 3 "Large"; default **Medium** (getter returns 2 when unset) | `SettingsStore.OverlaySize.displayName` 0x1000c2550, `SettingsStore.overlaySize` 0x1000c4b10 |
| Layout per size (Pill / Small / Medium / Large) | hPadding 12/10/18/18, vPadding 8/6/12/12, waveformHeight 30/20/32/48, transFontSize 10/11/13/15, modeFontSize 9/10/12/14, cornerRadius 23/14/18/24, barCount 8/7/8/11, barWidth 3/3/3.5/5, barSpacing 2.5/3.5/4.5/6, minBarHeight 4/5/6/8, maxBarHeight 28/16/28/44, containerWidth 100/200/340/600, overlayWidth 100/300/380/600, overlayHeight 46/124/156/288 (pt) | `BottomOverlayView.LayoutConstants.get(for:)` 0x100e9e4a4, tables 0x101a58800–0x101a589a0 |
| Preview text width | min(101.2/198/286/396, containerWidth − 2·hPadding) for sizes ≤ Medium → 76/180/286, Large 396 | `BottomOverlayView.previewMaxWidth` 0x100e9f984, tables 0x101a585e0, 0x101a58a60, 0x101a58640 |
| Preview length | last **150** characters (`TranscriptionPreviewCharLimit` default) | `SettingsStore.defaultTranscriptionPreviewCharLimit` 0x1000a4a94, `transcriptionPreviewCharLimit` 0x1000c56d8 |
| Look | black rounded panel with white level bars (onboarding picture, `BottomOverlayPanel`, `BottomWaveformView`); Pill size shows only the bars | strings + OnboardingSmartModeSelected.png + layout table (Pill: no preview) |
| This rebuild | window of overlayWidth×overlayHeight, panel containerWidth wide hugging the screen edge and growing with the text; centred on the screen in use (`NSScreen.mainScreen.visibleFrame`), always on top, never focused, click-through; red dot + "Écoute" + timer while recording, blue bars + "Transcription…" while transcribing, then the result (or a warning) for 1.8 s (**estimated**: FluidVoice streams the text live instead) | verified: Finder stayed frontmost while the pill was shown |
| Background use | closing the main window minimises it instead of quitting; "Quitter Voix Fluide" button quits | — |

## Output

| Rule | Value | Source |
|---|---|---|
| Default insertion | "Clipboard Paste (Recommended)": temporary clipboard entry, Cmd+V, clipboard restored | strings near `TypingService` / `PasteDeliveryCoordinator` |
| Copy result to clipboard as backup | off by default (`boolForKey` → false) | `SettingsStore.copyTranscriptionToClipboard` 0x1000be9a0 |
| Typing requires | Accessibility permission | Info.plist `NSAccessibilityUsageDescription` |
| Wait for the shortcut key to be let go before pasting | up to **2.0 s** (`fmov d0, #2.0` passed as `timeout`) | `TypingService.typeOutputPlanInstantly` closure 0x1007e59d8 |
| Poll interval of that wait | **15 ms** (`Task.sleep(nanoseconds: 0xE4E1C0)`), reads `CGEventSourceFlagsState` | `TypingService.waitForPhysicalModifiersToRelease` 0x1007ed560 |
| If still held after the timeout | paste aborted ("Paste aborted - modifier keys still held after timeout"), text kept as clipboard backup | 0x1007ed560 log + `PasteDeliveryCoordinator.copyBackup` called from 0x1007e59d8 |
| Delay before giving the clipboard back | **500 ms** (0x1DCD6500 ns) | `PasteDeliveryCoordinator.defaultSettlementDelayNanoseconds` 0x1007367e8 |
| Clipboard restore rule | only if the clipboard is unchanged since the temporary write | `SystemPasteboardManager.restore(_:ifUnchangedSince:)` 0x100733bc0 |
| Paste key | Cmd + the layout's "V" key (resolved through `PasteKeyCodeCache`); this rebuild uses key code 9 (V on QWERTY and AZERTY) — estimated | `TypingService.pasteVirtualKeyCode` 0x1007e3abc |
| Parakeet v3 catalog entry | id `parakeet-tdt-0.6b-v3-coreml`, name "Parakeet TDT v3 (Multilingual)", tagline "Blazing Fast - Multilingual", "25 Languages", size "~460.9 MiB" (CoreML build; the ONNX int8 build used here is ~670 MB), vocab `parakeet_vocab.json` | `ParakeetSpeechModelCatalog.v3` initializer 0x100724ec0 |

## Differences in this rebuild

- Whisper runs through the pure-Rust candle library with the openai/whisper safetensors weights (downloaded on first use) instead of whisper.cpp ggml files; sizes offered: Tiny, Base, Small.
- Start/stop sounds are short synthesized tones (the original's own sound files are not copied) — estimated.
- Look: dark panels and a bar level meter from the onboarding picture; the main window could not be captured — estimated.
- Left out on purpose (request "juste du STT"): AI rewrite/command/prompt modes, meetings, file transcription, Apple/Cohere engines, statistics.

## NVIDIA Parakeet (added on request)

| Rule | Value | Source |
|---|---|---|
| Model the original uses | "Parakeet TDT v3 (Multilingual)" — "uses CoreML and Neural Engine for fastest transcription (25 languages)"; repo `FluidInference/parakeet-tdt-0.6b-v3-coreml` | strings in the FluidVoice binary |
| Language | detected by the model itself (no language picker for Parakeet) | string "Parakeet TDT v2 handles English; v3 handles the supported European languages." |
| This rebuild | same model in ONNX form (`istupakov/parakeet-tdt-0.6b-v3-onnx`, int8 encoder + decoder + vocab, ~670 MB), run on the processor with ONNX Runtime — estimated equivalent of the CoreML build | test: 5.6 s French sentence transcribed word-for-word in ~0.9 s |

## Permissions that survive updates (this rebuild)

- macOS keys Accessibility and Microphone permissions to the program's signature. `cargo build` signs "ad hoc" (`designated => cdhash …`, identifier `voix_fluide-<hash>`), so every rebuild looked like a new app: the switch in Settings stayed on but applied to an older build, and `AXIsProcessTrusted()` returned false ("Texte copié — autorisez l'accessibilité…").
- At launch the program copies itself to `~/Library/Application Support/VoixFluide/programme/voix-fluide` (only when the build changed), signs the copy with the first keychain certificate whose signed copy actually starts (Developer ID Application first, then Apple Development; a revoked certificate gets the program killed with SIGKILL, measured), with identifier `app.voixfluide.dictee`, and runs that copy. Its designated requirement is then `identifier "app.voixfluide.dictee" and anchor apple generic and …team…`, the same for every build.
- FluidVoice itself asks for Accessibility "to listen for global hotkeys and type text into other apps" (Info.plist `NSAccessibilityUsageDescription`). The window now shows a "Réautoriser" button that calls `AXIsProcessTrustedWithOptions(prompt)` and opens the Accessibility settings.

## Accessibility permission flow

| Rule | Value | Source |
|---|---|---|
| Permission check | `AXIsProcessTrusted()` | `ContentView.checkAccessibilityPermissions` 0x100fc8764 |
| Re-check while waiting | every **2 s** (`Task.sleep(nanoseconds: 0x77359400)`), loop until trusted or cancelled | `ContentView.startAccessibilityPolling` closure 0x100fcb98c / 0x100fcba60 |
| When access comes back | logs "Accessibility permission granted", refreshes state, finishes the permission flow; hotkeys reinitialise ("Accessibility enabled, reinitializing hotkey manager") | 0x100fcbb94 |
| Settings button | opens `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`, then starts the 2 s polling | `ContentView.openAccessibilitySettings` 0x100f9b5f4 |
| System prompt | FluidVoice never calls `AXIsProcessTrustedWithOptions` (not in its code); its global event tap gets it listed. This rebuild has no event tap, so "Réautoriser" calls `AXIsProcessTrustedWithOptions(prompt: true)` to get listed — deviation | disassembly search |
| This rebuild | polls every 2 s (`AX_POLL_S`), shows "Accessibilité autorisée…" when access comes back | — |

## Mini-sessions Claude Code (added in this rebuild)

Not in FluidVoice: a voice remote for Claude Code (issue #1). Every value here is a choice of this rebuild.

| Rule | Value | Source |
|---|---|---|
| Mini-session key | second shortcut, default **⌘ Commande droite** (keyCode 54); never the dictation key (the settings refuse it, and a saved equal pair is corrected at load) | `Settings::session_key` |
| Recording | same as dictation (mode, minimum length, silence checks); while one key records, the other key is ignored | `controller`, `Target` |
| Repository | a folder of the root (default `~/Documents/c2-poly/repo`) whose name appears in the sentence as whole words, case and accents ignored, `-`/`_` read as spaces, hidden folders skipped, longest name wins (a folder that is not a Git repository but holds exactly one, like a workspace with its code in `app/`, stands for that one); else the last repository used; else "Dites dans quel dépôt…" and nothing runs | `claude::route` |
| Command | `claude -p <text> --tools Read,Grep,Glob --output-format json --setting-sources project --json-schema {resume, reponse} --append-system-prompt <answer in French>`, run in the repository; the session is kept on disk for `--resume` | `claude::args` |
| Finding `claude` | PATH, then `~/.local/bin`, `~/.claude/local`, `/opt/homebrew/bin`, `/usr/local/bin` (an app opened from the Finder has a short PATH) | `claude::find_claude` |
| Output | never pasted, the clipboard untouched; the window lists the mini-sessions (newest first), "Reprendre au terminal" copies `cd <repo> && claude --resume <id>` | `start_session`, Mini-sessions card |
| Pill | label "Claude", text "<dépôt> · en cours" while it runs, then "terminé" and the summary (or the error) for **6 s** (estimated) | `claude_note`, `CLAUDE_LINGER_S` |
| Read aloud | the one-sentence summary through `say`, setting "Lire le résumé à voix haute", on by default | `Settings::speak` |
