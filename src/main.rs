mod claude;
mod engine;
mod icon;
mod mac;
mod tray;
mod update;
mod whisper;

use egui::{Align, Color32, CornerRadius, FontId, Layout, Margin, RichText, Sense, Stroke, Vec2};
use engine::{Cmd, Handles, Job, MiniSession, Mode, ModelState, OverlayScreen, Phase, SessionState, Settings, SharedRef, Target, WaveStyle, LANGUAGES};
use std::path::PathBuf;
use update::UpdateState;
use whisper::ModelSize;

const BG_TOP: Color32 = Color32::from_rgb(5, 7, 13);
const BG_BOTTOM: Color32 = Color32::from_rgb(9, 24, 48);
const PANEL: Color32 = Color32::from_rgb(0x0E, 0x12, 0x1C);
const PANEL_STROKE: Color32 = Color32::from_rgb(0x26, 0x2C, 0x3C);
const BTN_FILL: Color32 = Color32::from_rgb(0x14, 0x19, 0x25);
const BTN_STROKE: Color32 = Color32::from_rgb(0x2E, 0x35, 0x46);
const SOFT: Color32 = Color32::from_rgb(0xC9, 0xCE, 0xD8);
const KEY_FILL: Color32 = Color32::from_rgb(0x16, 0x1B, 0x27);
const KEY_STROKE: Color32 = Color32::from_rgb(0x3A, 0x41, 0x52);
const TEXT: Color32 = Color32::from_rgb(236, 238, 242);
const MUTED: Color32 = Color32::from_rgb(140, 146, 160);
const ACCENT: Color32 = Color32::from_rgb(0x1A, 0x75, 0xFF);
const RED: Color32 = Color32::from_rgb(255, 69, 58);
const GREEN: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
const AMBER: Color32 = Color32::from_rgb(255, 196, 92);

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == mac::SIGNATURE_CHECK_ARG) {
        return Ok(());
    }
    if args.iter().any(|a| a == "--version") {
        println!("{}", version_line(env!("CARGO_PKG_VERSION"), env!("VF_COMMIT"), env!("VF_RELEASE") == "1"));
        return Ok(());
    }
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);

    // Command-line check of the transcription: --transcribe <16-bit WAV> [language]
    if let Some(i) = args.iter().position(|a| a == "--transcribe") {
        let path = args.get(i + 1).expect("--transcribe <file.wav>");
        let lang = args.get(i + 2).map(String::as_str);
        let size = Settings::load().model;
        if !whisper::is_downloaded(size) {
            eprintln!("downloading {}…", size.short());
            whisper::download(size, |_, _| {}).expect("download");
        }
        let (pcm, rate) = read_wav(path).expect("wav");
        let pcm = engine::resample(&pcm, rate);
        let t = std::time::Instant::now();
        let mut e = whisper::Engine::load(size, true).expect("load");
        let text = e.transcribe(&pcm, lang).expect("transcribe");
        println!("{text}");
        eprintln!("{:.2}s audio, {:.2}s total ({})", pcm.len() as f32 / 16000.0, t.elapsed().as_secs_f32(), if e.device.is_metal() { "GPU" } else { "CPU" });
        return Ok(());
    }

    // Answer for our own microphone permission, whoever started us.
    if screenshot.is_none() {
        mac::run_with_own_permissions();
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Voix Fluide")
            // Dock and Cmd+Tab icon: the app runs as a bare binary, so there is no .app bundle to carry it.
            .with_icon(std::sync::Arc::new(egui::IconData { rgba: icon::render(512, true), width: 512, height: 512 }))
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([760.0, 520.0])
            // Unified title bar: the content runs under a transparent title bar, so the
            // sidebar reaches the top and the traffic lights sit on it.
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            // Lets the floating pill have rounded, see-through corners: transparency is
            // chosen once for the GL context, so the root window must ask for it.
            .with_transparent(true),
        ..Default::default()
    };
    eframe::run_native(
        "Voix Fluide",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, screenshot)))),
    )
}

/// 16-bit PCM WAV reader (mono or mixed down), returns samples and rate.
fn read_wav(path: &str) -> Result<(Vec<f32>, u32), String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let (mut pos, mut rate, mut ch) = (12usize, 16000u32, 1usize);
    while pos + 8 <= b.len() {
        let id = &b[pos..pos + 4];
        let len = u32::from_le_bytes(b[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = &b[pos + 8..(pos + 8 + len).min(b.len())];
        if id == b"fmt " {
            ch = u16::from_le_bytes([body[2], body[3]]) as usize;
            rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
        } else if id == b"data" {
            let s: Vec<f32> = body
                .chunks_exact(2 * ch)
                .map(|f| {
                    f.chunks_exact(2)
                        .map(|x| i16::from_le_bytes([x[0], x[1]]) as f32 / 32768.0)
                        .sum::<f32>()
                        / ch as f32
                })
                .collect();
            return Ok((s, rate));
        }
        pos += 8 + len + (len & 1);
    }
    Err("no data chunk".into())
}

/// Use the Mac's own fonts when present: SF for text, Apple Symbols for ⌥ ⌘ ⌃.
fn fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    let mut add = |name: &str, path: &str, first: bool| {
        if let Ok(bytes) = std::fs::read(path) {
            defs.font_data
                .insert(name.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            let list = defs.families.entry(egui::FontFamily::Proportional).or_default();
            if first {
                list.insert(0, name.into());
            } else {
                list.push(name.into());
            }
        }
    };
    add("sf", "/System/Library/Fonts/SFNS.ttf", true);
    add("symbols", "/System/Library/Fonts/Apple Symbols.ttf", false);
    ctx.set_fonts(defs);
}

/// The pages of the window, one per entry of the left sidebar.
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    History,
    Claude,
    Keys,
    Model,
    Pill,
    Audio,
    Updates,
}

impl Page {
    /// VF_PAGE=<name> opens the window on that page (used by screenshots).
    fn from_env() -> Page {
        match std::env::var("VF_PAGE").as_deref() {
            Ok("history") => Page::History,
            Ok("claude") => Page::Claude,
            Ok("keys") => Page::Keys,
            Ok("model") => Page::Model,
            Ok("pill") => Page::Pill,
            Ok("audio") => Page::Audio,
            Ok("updates") => Page::Updates,
            _ => Page::Home,
        }
    }
}

struct App {
    /// The page shown on the right of the sidebar.
    page: Page,
    /// The logo mark shown in the header.
    logo: egui::TextureHandle,
    shared: SharedRef,
    handles: Option<Handles>,
    screenshot: Option<PathBuf>,
    frames: u32,
    shot_requested: bool,
    ax_trusted: bool,
    ax_checked: f64,
    mic_access: mac::MicAccess,
    devices: Vec<String>,
    default_device: Option<String>,
    prev_phase: Phase,
    /// VF_PILL_SHOT=<file.png>: draw only the pill and save its picture (test).
    pill_test: bool,
    quitting: bool,
    /// The pill stays up until this time after a dictation ends (to show the result).
    overlay_until: f64,
    overlay_note: Option<String>,
    overlay_pos: Option<egui::Pos2>,
    /// A short note at the bottom of the window and the time it goes away.
    toast: Option<(String, f64, bool)>,
    /// The icon and menu in the menu bar (none in screenshot runs).
    tray: Option<tray::Tray>,
    /// The Updates page's state, written by its own thread.
    update: update::UpdateRef,
}

/// Height of the macOS title bar the content now runs under, in points.
const TITLEBAR_PT: f32 = 22.0;

/// Height of the logo mark in the window header, in points.
const LOGO_PT: f32 = 28.0;

/// How long a toast stays at the bottom of the window (this rebuild).
const TOAST_S: f64 = 3.0;

/// ContentView.startAccessibilityPolling (0x100fcb98c / 0x100fcba60): checks
/// AXIsProcessTrusted, then Task.sleep(0x77359400 ns) = every 2 s.
const AX_POLL_S: f64 = 2.0;

/// FluidVoice SettingsStore.overlayBottomOffset (0x1000c48cc): 50 pt when unset.
const OVERLAY_OFFSET: f32 = 50.0;
/// How long the result stays visible in the pill after a dictation (estimated).
const OVERLAY_LINGER_S: f64 = 1.8;
/// SettingsStore.defaultTranscriptionPreviewCharLimit (0x1000a4a94): 150 characters.
const OVERLAY_PREVIEW_CHARS: usize = 150;
/// How long the pill shows a finished mini-session (this rebuild, estimated).
const CLAUDE_LINGER_S: f32 = 6.0;

/// A mini-session's state as the pill shows it.
#[derive(Clone, Copy, PartialEq)]
enum ClaudeTone {
    Running,
    Done,
    Failed,
}

/// The pill's line for the newest running mini-session, or for one that just ended.
fn claude_note(sessions: &[MiniSession]) -> Option<(String, ClaudeTone)> {
    let s = sessions
        .iter()
        .rev()
        .find(|s| s.state == SessionState::Running)
        .or_else(|| sessions.iter().rev().find(|s| s.finished.is_some_and(|t| t.elapsed().as_secs_f32() < CLAUDE_LINGER_S)))?;
    let name = s.repo_name();
    Some(match &s.state {
        // The pill's label already says "Claude".
        SessionState::Running => (format!("{name} · en cours"), ClaudeTone::Running),
        SessionState::Done { resume, .. } => (format!("{name} · terminé\n{resume}"), ClaudeTone::Done),
        SessionState::Failed(e) => (format!("{name} · {e}"), ClaudeTone::Failed),
    })
}

/// VF_SESSIONS=1 (screenshots): one running, one finished and one failed mini-session.
fn sample_sessions(root: &std::path::Path) -> Vec<MiniSession> {
    let now = std::time::Instant::now();
    let at = |id: u64, repo: &str, when: &str, question: &str, state: SessionState| MiniSession {
        id,
        repo: root.join(repo),
        question: question.into(),
        finished: (state != SessionState::Running).then_some(now),
        state,
        when: when.into(),
        started: now - std::time::Duration::from_secs(12),
    };
    vec![
        at(1, "fourchette", "09:12", "Dans Fourchette, quelle commande lance les tests de bout en bout ?", SessionState::Failed("Claude n'a pas pu répondre : Credit balance is too low".into())),
        at(2, "voix-fluide", "09:14", "Dans voix fluide, où est réglé le seuil entre appui court et appui long ?", SessionState::Done {
            session_id: "5f1c2a7e-8d34-4b9e-a0c1-3e2f6d7b9a10".into(),
            resume: "Le seuil de 0,4 seconde est la constante TAP_HOLD_THRESHOLD dans engine.rs.".into(),
            reponse: "Le seuil est `TAP_HOLD_THRESHOLD = 0.4` en haut de `src/engine.rs`. Le `controller` le compare à la durée de l'appui en mode Automatique : plus court, l'appui bascule l'enregistrement ; plus long, il enregistre tant que la touche est tenue.".into(),
        }),
        at(3, "fourchette", "09:15", "Et dans quel fichier sont gérés les worktrees ?", SessionState::Running),
    ]
}

/// One row of FluidVoice's BottomOverlayView.LayoutConstants.get(for:) (0x100e9e4a4),
/// tables at 0x101a58800..0x101a589a0; previewMaxWidth table 0x101a585e0 (0x100e9f984).
#[derive(Clone, Copy)]
struct PillLayout {
    h_padding: f32,
    v_padding: f32,
    waveform_height: f32,
    trans_font: f32,
    mode_font: f32,
    corner_radius: f32,
    bar_count: usize,
    bar_width: f32,
    bar_spacing: f32,
    min_bar: f32,
    max_bar: f32,
    container_width: f32,
    overlay_width: f32,
    overlay_height: f32,
    preview_max_width: f32,
    shows_preview: bool,
    shows_mode_label: bool,
}

/// OverlaySize: 0 Pill, 1 Small, 2 Medium (default, 0x1000c4b10), 3 Large (0x1000c2550).
const PILL_LAYOUTS: [PillLayout; 4] = [
    PillLayout { h_padding: 12.0, v_padding: 8.0, waveform_height: 30.0, trans_font: 10.0, mode_font: 9.0, corner_radius: 23.0, bar_count: 8, bar_width: 3.0, bar_spacing: 2.5, min_bar: 4.0, max_bar: 28.0, container_width: 100.0, overlay_width: 100.0, overlay_height: 46.0, preview_max_width: 76.0, shows_preview: false, shows_mode_label: true },
    PillLayout { h_padding: 10.0, v_padding: 6.0, waveform_height: 20.0, trans_font: 11.0, mode_font: 10.0, corner_radius: 14.0, bar_count: 7, bar_width: 3.0, bar_spacing: 3.5, min_bar: 5.0, max_bar: 16.0, container_width: 200.0, overlay_width: 300.0, overlay_height: 124.0, preview_max_width: 180.0, shows_preview: true, shows_mode_label: true },
    PillLayout { h_padding: 18.0, v_padding: 12.0, waveform_height: 32.0, trans_font: 13.0, mode_font: 12.0, corner_radius: 18.0, bar_count: 8, bar_width: 3.5, bar_spacing: 4.5, min_bar: 6.0, max_bar: 28.0, container_width: 340.0, overlay_width: 380.0, overlay_height: 156.0, preview_max_width: 286.0, shows_preview: true, shows_mode_label: true },
    PillLayout { h_padding: 18.0, v_padding: 12.0, waveform_height: 48.0, trans_font: 15.0, mode_font: 14.0, corner_radius: 24.0, bar_count: 11, bar_width: 5.0, bar_spacing: 6.0, min_bar: 8.0, max_bar: 44.0, container_width: 600.0, overlay_width: 600.0, overlay_height: 288.0, preview_max_width: 396.0, shows_preview: true, shows_mode_label: true },
];
const PILL_SIZE_NAMES: [&str; 4] = ["Pastille", "Petite", "Moyenne", "Grande"];

fn pill_layout(size: u8) -> PillLayout {
    PILL_LAYOUTS[(size as usize).min(3)]
}

/// The pill's look chosen in the settings, passed to `draw_pill` so the floating pill,
/// the preview on the Pastille page and `VF_PILL_SHOT` draw it the same way.
#[derive(Clone, Copy)]
struct PillStyle {
    wave: WaveStyle,
    timer: bool,
}

impl PillStyle {
    fn of(settings: &Settings) -> Self {
        PillStyle { wave: settings.overlay_wave, timer: settings.overlay_timer }
    }
}

/// How long each state stays in the preview of the Pastille page.
const PREVIEW_STEP_S: f64 = 2.5;
const PREVIEW_TEXT: &str = "Bonjour, ceci est un essai de dictée vocale. Le texte apparaît là où se trouve le curseur.";
const PREVIEW_CLAUDE: &str = "voix-fluide · terminé\nLe seuil de 0,4 seconde est TAP_HOLD_THRESHOLD dans engine.rs.";

/// A smoothed line through `heights` (one per bar, centred on `x0 + i * step`):
/// the points swing above and below `cy` in turn, like a sound wave, and the line
/// starts and ends on `cy` half a step outside the first and last points.
fn wave_line(p: &egui::Painter, x0: f32, step: f32, cy: f32, heights: &[f32], stroke: impl Into<egui::epaint::PathStroke>) {
    if heights.is_empty() {
        return;
    }
    let mut ctrl = vec![egui::pos2(x0 - step / 2.0, cy)];
    for (i, h) in heights.iter().enumerate() {
        let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
        ctrl.push(egui::pos2(x0 + i as f32 * step, cy + sign * h / 2.0));
    }
    ctrl.push(egui::pos2(x0 + (heights.len() as f32 - 0.5) * step, cy));
    // Catmull-Rom through the control points, 8 samples per segment.
    let at = |i: isize| ctrl[i.clamp(0, ctrl.len() as isize - 1) as usize].to_vec2();
    let mut pts = Vec::with_capacity(ctrl.len() * 8);
    for i in 0..ctrl.len() as isize - 1 {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        for k in 0..8 {
            let t = k as f32 / 8.0;
            let (t2, t3) = (t * t, t * t * t);
            let v = (p1 * 2.0 + (p2 - p0) * t + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2 + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3) * 0.5;
            pts.push(v.to_pos2());
        }
    }
    pts.push(*ctrl.last().unwrap());
    p.add(egui::Shape::line(pts, stroke));
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        fonts(&cc.egui_ctx);
        style(&cc.egui_ctx);
        let settings = Settings::load();
        let shared = engine::new_shared(settings);
        if std::env::var_os("VF_SESSIONS").is_some() {
            let mut sh = shared.lock().unwrap();
            sh.sessions = sample_sessions(&claude::expand_home(&sh.settings.repo_root));
        }
        // The screenshot run only draws the window: no shortcut watcher, no model loading.
        let pill_test = std::env::var_os("VF_PILL_SHOT").map(PathBuf::from);
        let screenshot = pill_test.clone().or(screenshot);
        let handles = if pill_test.is_some() {
            // Test mode: show the pill as if dictating, save its picture, quit.
            let mut sh = shared.lock().unwrap();
            sh.phase = Phase::Recording;
            sh.record_started = Some(std::time::Instant::now() - std::time::Duration::from_secs(7));
            for (i, l) in sh.levels.iter_mut().enumerate() {
                *l = 0.25 + 0.6 * ((i as f32 * 0.9).sin().abs());
            }
            None
        } else if screenshot.is_none() {
            // Like FluidVoice's onboarding: ask for the microphone right away.
            if mac::mic_access() == mac::MicAccess::NotAsked {
                mac::request_mic_access();
            }
            Some(engine::start(shared.clone(), cc.egui_ctx.clone()))
        } else {
            let mut sh = shared.lock().unwrap();
            if sh.model == ModelState::Loading {
                sh.model = ModelState::Ready(format!("{} · {}", sh.settings.model.short(), if sh.settings.model.is_parakeet() { "CPU" } else { "GPU" }));
            }
            drop(sh);
            None
        };
        let tray = if handles.is_some() {
            tray::Tray::new(&cc.egui_ctx).map_err(|e| eprintln!("menu bar icon: {e}")).ok()
        } else {
            None
        };
        let logo_px = ((LOGO_PT * cc.egui_ctx.pixels_per_point()).ceil() as u32).max(56);
        let logo = cc.egui_ctx.load_texture(
            "logo-mark",
            egui::ColorImage::from_rgba_unmultiplied([logo_px as usize; 2], &icon::render(logo_px, false)),
            egui::TextureOptions::LINEAR,
        );
        // VF_UPDATE=<state> (screenshots): a fake state, no network.
        let fake = std::env::var("VF_UPDATE").ok().and_then(|s| update::fake_state(&s));
        let update = std::sync::Arc::new(std::sync::Mutex::new(fake.clone().unwrap_or(UpdateState::Idle)));
        if fake.is_none() && update::check_at_launch(env!("VF_RELEASE") == "1", screenshot.is_some()) {
            update::spawn_check(update.clone(), cc.egui_ctx.clone());
        }
        Self {
            page: Page::from_env(),
            logo,
            shared,
            handles,
            screenshot,
            frames: 0,
            shot_requested: false,
            ax_trusted: mac::accessibility_trusted(),
            ax_checked: 0.0,
            mic_access: mac::MicAccess::Allowed,
            devices: Vec::new(),
            default_device: None,
            prev_phase: Phase::Idle,
            pill_test: pill_test.is_some(),
            quitting: false,
            overlay_until: 0.0,
            overlay_note: None,
            overlay_pos: None,
            // VF_TOAST=<text> (screenshots): show a toast that never goes away.
            toast: std::env::var("VF_TOAST").ok().map(|t| (t, f64::INFINITY, true)),
            tray,
            update,
        }
    }

    fn send(&self, c: Cmd) {
        if let Some(h) = &self.handles {
            let _ = h.cmd.send(c);
        }
    }

    fn job(&self, j: Job) {
        if let Some(h) = &self.handles {
            let _ = h.job.send(j);
        }
    }
}

fn style(ctx: &egui::Context) {
    // The window is always dark, whatever the macOS appearance is.
    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = PANEL;
    v.window_stroke = Stroke::new(1.0_f32, PANEL_STROKE);
    v.extreme_bg_color = Color32::from_rgb(0x0A, 0x0D, 0x15);
    v.faint_bg_color = BTN_FILL;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT.linear_multiply(0.6);
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.weak_bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, PANEL_STROKE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.inactive.bg_fill = BTN_FILL;
    v.widgets.inactive.weak_bg_fill = BTN_FILL;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BTN_STROKE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(0x1C, 0x23, 0x33);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x1C, 0x23, 0x33);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x4A, 0x53, 0x68));
    v.widgets.active.bg_fill = Color32::from_rgb(0x24, 0x2D, 0x42);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x24, 0x2D, 0x42);
    v.widgets.open.bg_fill = BTN_FILL;
    v.widgets.open.weak_bg_fill = BTN_FILL;
    v.widgets.open.bg_stroke = Stroke::new(1.0_f32, BTN_STROKE);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(8);
    }
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(12.0, 6.0);
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.0));
    });
}

/// Secondary button: dark fill, thin stroke, soft text.
fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(SOFT))
            .fill(BTN_FILL)
            .stroke(Stroke::new(1.0_f32, BTN_STROKE))
            .corner_radius(CornerRadius::same(8)),
    )
}

/// Short form of a shortcut label: "⌥ Option droite" becomes "⌥ droite".
fn short_key(label: &str) -> String {
    let words: Vec<&str> = label.split_whitespace().collect();
    match words.as_slice() {
        [first, .., last] => format!("{first} {last}"),
        _ => label.to_string(),
    }
}

/// A keycap drawn inline: rounded rect with a thicker bottom edge, text on the baseline.
fn keycap(ui: &mut egui::Ui, text: &str) {
    let font = FontId::proportional(12.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, SOFT);
    let size = Vec2::new(galley.size().x + 14.0, galley.size().y + 4.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    let r = CornerRadius::same(6);
    p.rect_filled(rect.translate(Vec2::new(0.0, 1.5)), r, KEY_STROKE);
    p.rect_filled(rect, r, KEY_FILL);
    p.rect_stroke(rect, r, Stroke::new(1.0_f32, KEY_STROKE), egui::StrokeKind::Inside);
    p.galley(rect.center() - galley.size() / 2.0, galley, SOFT);
}

/// A centred line of text with keycaps: `parts` alternate plain text and keycap text, starting with text.
fn key_line(ui: &mut egui::Ui, parts: &[&str]) {
    let gap = 5.0;
    let width_of = |ui: &egui::Ui, text: &str, cap: bool| {
        let w = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(if cap { 12.0 } else { 13.0 }), SOFT).size().x;
        if cap { w + 14.0 } else { w }
    };
    let shown: Vec<(usize, &&str)> = parts.iter().enumerate().filter(|(_, t)| !t.is_empty()).collect();
    let total: f32 = shown.iter().map(|(i, t)| width_of(ui, t, i % 2 == 1)).sum::<f32>() + gap * (shown.len().saturating_sub(1)) as f32;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
        for (i, part) in shown {
            if i % 2 == 0 {
                ui.label(RichText::new(*part).size(13.0).color(MUTED));
            } else {
                keycap(ui, part);
            }
        }
    });
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, PANEL_STROKE))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
}

fn caps(ui: &mut egui::Ui, s: &str) {
    ui.label(RichText::new(s).size(11.5).strong().color(MUTED));
}

/// A row of the sidebar: label, accent bar when selected, optional badge.
fn nav_item(ui: &mut egui::Ui, label: &str, active: bool, badge: Option<&str>) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
    let p = ui.painter();
    let r = CornerRadius::same(9);
    if active {
        p.rect_filled(rect, r, Color32::from_rgb(0x15, 0x23, 0x3F));
        p.rect_filled(egui::Rect::from_min_size(rect.left_top() + Vec2::new(0.0, 7.0), Vec2::new(2.5, 20.0)), CornerRadius::same(2), ACCENT);
    } else if resp.hovered() {
        p.rect_filled(rect, r, Color32::from_rgb(0x10, 0x16, 0x26));
    }
    let col = if active { Color32::WHITE } else { Color32::from_rgb(0xB4, 0xBA, 0xC8) };
    p.text(rect.left_center() + Vec2::new(14.0, 0.0), egui::Align2::LEFT_CENTER, label, FontId::proportional(14.0), col);
    if let Some(b) = badge {
        let g = p.layout_no_wrap(b.to_string(), FontId::proportional(11.0), Color32::from_rgb(0x6F, 0xA8, 0xFF));
        let br = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 16.0 - g.size().x / 2.0, rect.center().y),
            g.size() + Vec2::new(14.0, 4.0),
        );
        p.rect_filled(br, CornerRadius::same(6), Color32::from_rgb(0x0D, 0x22, 0x44));
        p.galley(br.center() - g.size() / 2.0, g, Color32::WHITE);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Title and subtitle at the top of a page.
fn page_header(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(RichText::new(title).size(24.0).strong().color(TEXT));
    ui.label(RichText::new(sub).color(MUTED));
    ui.add_space(8.0);
}

/// A row of a settings card: title and help on the left, the control on the right.
fn setting_row(ui: &mut egui::Ui, title: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    let w = ui.available_width();
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new((w - 250.0).max(120.0), 0.0), Layout::top_down(Align::Min), |ui| {
            ui.label(RichText::new(title).color(TEXT));
            if !help.is_empty() {
                ui.add(egui::Label::new(RichText::new(help).size(12.0).color(MUTED)).wrap());
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
}

fn row_separator(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.separator();
    ui.add_space(2.0);
}

/// An on/off switch.
fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(38.0, 22.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(11), if *on { ACCENT } else { Color32::from_rgb(0x2A, 0x30, 0x42) });
    let x = if *on { rect.right() - 11.0 } else { rect.left() + 11.0 };
    p.circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A choice shown as a card (title and help): the selected one is outlined in blue.
fn choice_card(ui: &mut egui::Ui, width: f32, selected: bool, title: &str, help: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 84.0), Sense::click());
    let p = ui.painter();
    let (fill, stroke) = if selected { (Color32::from_rgb(0x15, 0x23, 0x3F), ACCENT) } else { (Color32::from_rgb(0x0A, 0x0D, 0x15), PANEL_STROKE) };
    p.rect_filled(rect, CornerRadius::same(12), fill);
    p.rect_stroke(rect, CornerRadius::same(12), Stroke::new(1.0_f32, stroke), egui::StrokeKind::Inside);
    p.text(rect.left_top() + Vec2::new(14.0, 14.0), egui::Align2::LEFT_TOP, title, FontId::proportional(14.0), TEXT);
    let g = p.layout(help.to_string(), FontId::proportional(12.0), if selected { SOFT } else { MUTED }, width - 28.0);
    p.galley(rect.left_top() + Vec2::new(14.0, 36.0), g, MUTED);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// An amber warning card with a title, a text and buttons.
fn warn_card(ui: &mut egui::Ui, title: &str, body: &str, buttons: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::from_rgb(58, 34, 10))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(150, 96, 30)))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).strong().color(Color32::from_rgb(255, 205, 120)));
            ui.label(RichText::new(body).size(13.0));
            ui.horizontal(buttons);
        });
    ui.add_space(12.0);
}

fn paint_background(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), BG_TOP);
    mesh.colored_vertex(rect.right_top(), BG_TOP);
    mesh.colored_vertex(rect.right_bottom(), BG_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), BG_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

impl App {
    /// The small floating window that appears anywhere while dictating, without
    /// opening the main window. It never takes the keyboard or clicks, so the
    /// text still lands in the app the person is writing in.
    #[allow(clippy::too_many_arguments)]
    fn overlay(&mut self, ctx: &egui::Context, settings: &Settings, phase: Phase, target: Target, levels: &[f32], elapsed: Option<f32>, last: &str, message: &Option<String>, sessions: &[MiniSession], now: f64) {
        if self.screenshot.is_some() || !settings.overlay {
            self.prev_phase = phase;
            return;
        }
        // Keep the result on screen for a moment once a dictation ends.
        if phase == Phase::Idle && self.prev_phase != Phase::Idle {
            self.overlay_until = now + OVERLAY_LINGER_S;
            self.overlay_note = match message {
                Some(m) => Some(m.clone()),
                // A question for Claude: the mini-session's state takes over.
                None if target == Target::Session => None,
                // The text is hidden: keep the "Collé" header alone.
                None if !last.is_empty() && !settings.overlay_text => Some(String::new()),
                None if !last.is_empty() => {
                    let n = last.chars().count();
                    Some(if n > OVERLAY_PREVIEW_CHARS {
                        format!("…{}", last.chars().skip(n - OVERLAY_PREVIEW_CHARS).collect::<String>())
                    } else {
                        last.to_string()
                    })
                }
                None => None,
            };
        }
        if phase != Phase::Idle && self.prev_phase == Phase::Idle {
            // New dictation: place the pill on the screen being used right now.
            let l = pill_layout(settings.overlay_size);
            self.overlay_pos = mac::overlay_position(l.overlay_width, l.overlay_height, OVERLAY_OFFSET, settings.overlay_top, settings.overlay_screen)
                .map(|(x, y)| egui::pos2(x, y));
            self.overlay_note = None;
        }
        self.prev_phase = phase;
        let lingering = phase == Phase::Idle && now < self.overlay_until && self.overlay_note.is_some();
        let claude = if phase == Phase::Idle && !lingering { claude_note(sessions) } else { None };
        if phase == Phase::Idle && !lingering && claude.is_none() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));

        let mut builder = egui::ViewportBuilder::default()
            .with_title("Voix Fluide – dictée")
            .with_inner_size([pill_layout(settings.overlay_size).overlay_width, pill_layout(settings.overlay_size).overlay_height])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_active(false)
            .with_mouse_passthrough(true)
            .with_taskbar(false);
        if let Some(p) = self.overlay_pos {
            builder = builder.with_position(p);
        }
        let note = self.overlay_note.clone();
        let has_message = message.is_some();
        let layout = pill_layout(settings.overlay_size);
        let top = settings.overlay_top;
        let style = PillStyle::of(settings);
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("voix-fluide-pill"), builder, |ctx, _| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    let claude = claude.as_ref().map(|(t, tone)| (t.as_str(), *tone));
                    draw_pill(ui, ui.max_rect(), layout, style, top, phase, levels, elapsed, note.as_deref(), has_message, claude, now);
                });
        });
    }
}

/// The pill's look, from FluidVoice's layout table: a rounded panel `container_width`
/// wide hugging the screen edge, the waveform, a small state label and time, and the
/// text preview (last 150 characters) when the size shows one. With `claude` and no
/// dictation under way, it shows that mini-session instead. An empty `note` shows
/// the "Collé" header without the text.
#[allow(clippy::too_many_arguments)]
fn draw_pill(ui: &egui::Ui, canvas: egui::Rect, l: PillLayout, style: PillStyle, top: bool, phase: Phase, levels: &[f32], elapsed: Option<f32>, note: Option<&str>, has_message: bool, claude: Option<(&str, ClaudeTone)>, now: f64) {
    let p = ui.painter();
    let claude = claude.filter(|_| phase == Phase::Idle);
    let pill_mode = !l.shows_preview;
    let text = note.filter(|n| !n.is_empty());

    // Text under the header, as today: it sets the pill's height, so the height never changes.
    let sizing: Option<String> = if pill_mode {
        None
    } else {
        match phase {
            Phase::Recording => None,
            Phase::Transcribing => Some("Transcription…".to_string()),
            Phase::Idle => match claude {
                Some((t, _)) => Some(t.to_string()),
                None => text.map(str::to_string),
            },
        }
    };
    let sizing_galley = sizing.map(|t| p.layout(t, FontId::proportional(l.trans_font), TEXT, l.preview_max_width));
    let text_h = sizing_galley.as_ref().map(|g| g.size().y + 6.0).unwrap_or(0.0);
    let head_h = if pill_mode { l.overlay_height } else { l.v_padding * 2.0 + l.waveform_height };
    let height = (head_h + text_h).min(canvas.height());
    let x0 = canvas.center().x - l.container_width / 2.0;
    let y0 = if top { canvas.top() } else { canvas.bottom() - height };
    let rect = egui::Rect::from_min_size(egui::pos2(x0, y0), Vec2::new(l.container_width, height)).shrink(0.5);
    let stroke = if phase == Phase::Transcribing { Color32::from_rgb(0x1A, 0x3A, 0x6E) } else { Color32::from_rgb(0x2E, 0x35, 0x46) };
    p.rect(rect, CornerRadius::same(l.corner_radius as u8), Color32::from_rgba_unmultiplied(0, 0, 0, 235), Stroke::new(1.0_f32, stroke), egui::StrokeKind::Inside);
    let wave_cy = rect.top() + head_h / 2.0;
    let bar_radius = CornerRadius::same((l.bar_width / 2.0).ceil() as u8);
    let total = l.bar_count as f32 * l.bar_width + (l.bar_count as f32 - 1.0) * l.bar_spacing;

    // A bump travelling left to right over `n` bars: 1 under it, fading with the distance.
    let wave = |i: usize, n: usize| -> f32 {
        let pos = (now as f32 * 2.2) % (n as f32 + 3.0) - 1.5;
        let d = i as f32 - pos;
        (-(d * d) / 2.0).exp()
    };
    let bar = |cx: f32, cy: f32, w: f32, h: f32, col: Color32, radius: CornerRadius| {
        p.rect_filled(egui::Rect::from_center_size(egui::pos2(cx, cy), Vec2::new(w, h)), radius, col);
    };

    let lx = rect.left() + l.h_padding;
    let green_text = Color32::from_rgb(0x5B, 0xD9, 0x8A);

    match phase {
        Phase::Recording => {
            let bx0 = rect.center().x - total / 2.0;
            let start = levels.len().saturating_sub(l.bar_count);
            let heights: Vec<f32> = (0..l.bar_count)
                .map(|i| l.min_bar + levels.get(start + i).copied().unwrap_or(0.0).clamp(0.0, 1.0) * (l.max_bar - l.min_bar))
                .collect();
            match style.wave {
                WaveStyle::Bars => {
                    for (i, h) in heights.iter().enumerate() {
                        bar(bx0 + i as f32 * (l.bar_width + l.bar_spacing) + l.bar_width / 2.0, wave_cy, l.bar_width, *h, Color32::WHITE, bar_radius);
                    }
                }
                WaveStyle::Line => {
                    wave_line(p, bx0 + l.bar_width / 2.0, l.bar_width + l.bar_spacing, wave_cy, &heights, Stroke::new(2.0_f32, Color32::WHITE));
                }
            }
            if l.shows_mode_label && !pill_mode {
                // The logo's ending: a flat dash, then the blue caret.
                let dash_x = bx0 + total + l.bar_spacing + 2.0;
                let dash_w = l.bar_width * 2.5;
                bar(dash_x + dash_w / 2.0, wave_cy, dash_w, l.bar_width, Color32::WHITE, CornerRadius::same((l.bar_width / 2.0).ceil() as u8));
                let caret_h = (l.max_bar * 0.6).max(10.0);
                bar(dash_x + dash_w + l.bar_spacing + 1.0, wave_cy, 2.0, caret_h, ACCENT, CornerRadius::same(1));

                let pulse = 0.75 + 0.25 * ((now * 4.0).sin() as f32);
                let c = egui::pos2(lx + 5.0, wave_cy);
                p.circle_filled(c, 4.5 + 3.0 * pulse, Color32::from_rgba_unmultiplied(RED.r(), RED.g(), RED.b(), (70.0 * pulse) as u8));
                p.circle_filled(c, 4.0 * pulse, RED);
                // Only when there is room beside the waveform.
                if (bx0 - lx - 13.0) > 70.0 {
                    p.text(egui::pos2(lx + 15.0, wave_cy), egui::Align2::LEFT_CENTER, "Écoute", FontId::proportional(l.mode_font), MUTED);
                }
                let secs = elapsed.unwrap_or(0.0) as u32;
                if style.timer {
                    p.text(egui::pos2(rect.right() - l.h_padding, wave_cy), egui::Align2::RIGHT_CENTER, format!("{}:{:02}", secs / 60, secs % 60), FontId::monospace(l.mode_font), MUTED);
                }
            }
        }
        Phase::Transcribing => {
            let light_blue = Color32::from_rgb(0x9C, 0xC2, 0xFF);
            let label = (!pill_mode).then(|| p.layout("Transcription…".to_string(), FontId::proportional(l.trans_font), light_blue, l.preview_max_width));
            let gap = 10.0;
            let inline = label.as_ref().is_some_and(|g| total + gap + g.size().x <= l.container_width - 2.0 * l.h_padding);
            let group_w = total + label.as_ref().filter(|_| inline).map_or(0.0, |g| gap + g.size().x);
            let (cy, bx0) = if inline { (rect.center().y, rect.center().x - group_w / 2.0) } else { (wave_cy, rect.center().x - total / 2.0) };
            let step = l.bar_width + l.bar_spacing;
            match style.wave {
                WaveStyle::Bars => {
                    for i in 0..l.bar_count {
                        let k = wave(i, l.bar_count);
                        let h = l.min_bar + (0.25 + 0.75 * k) * (l.max_bar - l.min_bar) * 0.8;
                        bar(bx0 + i as f32 * step + l.bar_width / 2.0, cy, l.bar_width, h, ACCENT.gamma_multiply(0.3 + 0.7 * k), bar_radius);
                    }
                }
                WaveStyle::Line => {
                    let heights: Vec<f32> = (0..l.bar_count).map(|i| l.min_bar + (0.25 + 0.75 * wave(i, l.bar_count)) * (l.max_bar - l.min_bar) * 0.8).collect();
                    // The same travelling bump as the bars, read along the line.
                    let (x0, n, pos) = (bx0 + l.bar_width / 2.0, l.bar_count as f32, (now as f32 * 2.2) % (l.bar_count as f32 + 3.0) - 1.5);
                    let color = move |_: egui::Rect, q: egui::Pos2| {
                        let d = ((q.x - x0) / step).clamp(-0.5, n - 0.5) - pos;
                        ACCENT.gamma_multiply(0.3 + 0.7 * (-(d * d) / 2.0).exp())
                    };
                    wave_line(p, x0, step, cy, &heights, egui::epaint::PathStroke::new_uv(2.0_f32, color));
                }
            }
            if let Some(g) = label {
                let pos = if inline {
                    egui::pos2(bx0 + total + gap, cy - g.size().y / 2.0)
                } else {
                    egui::pos2(rect.center().x - g.size().x / 2.0, rect.top() + head_h - l.v_padding / 2.0)
                };
                p.galley(pos, g, light_blue);
            }
        }
        Phase::Idle if pill_mode => {
            // The minimal state: bars only.
            let bx0 = rect.center().x - total / 2.0;
            for i in 0..l.bar_count {
                bar(bx0 + i as f32 * (l.bar_width + l.bar_spacing) + l.bar_width / 2.0, wave_cy, l.bar_width, l.min_bar, if has_message { AMBER } else { GREEN }, bar_radius);
            }
        }
        Phase::Idle => {
            // Header row: a badge circle and a title (or "Claude · repo" and a state badge), then the text on the same left edge.
            let font = FontId::proportional(11.0);
            let (body, body_color): (Option<String>, Color32) = match claude {
                Some((t, tone)) => {
                    let body = match tone {
                        ClaudeTone::Running => None,
                        ClaudeTone::Done => t.split_once('\n').map(|(_, b)| b.to_string()),
                        ClaudeTone::Failed => t.split_once(" · ").map(|(_, e)| e.to_string()),
                    };
                    (body, TEXT)
                }
                None => (text.map(str::to_string), if has_message { Color32::from_rgb(0xFF, 0xE2, 0xA8) } else { TEXT }),
            };
            let cy = wave_cy;
            if let Some((t, tone)) = claude {
                let repo = t.split(" · ").next().unwrap_or("");
                let title = p.layout_no_wrap(format!("Claude · {repo}"), font.clone(), MUTED);
                let tw = title.size().x.min(l.container_width - 2.0 * l.h_padding - 60.0);
                p.galley(egui::pos2(lx, cy - title.size().y / 2.0), title, MUTED);
                let (fill, fg, label) = match tone {
                    ClaudeTone::Done => (Color32::from_rgb(0x0F, 0x2A, 0x1A), green_text, "terminé"),
                    ClaudeTone::Running => (Color32::from_rgb(0x0D, 0x22, 0x44), Color32::from_rgb(0x6F, 0xA8, 0xFF), "en cours"),
                    ClaudeTone::Failed => (Color32::from_rgb(0x3A, 0x22, 0x08), AMBER, "erreur"),
                };
                let g = p.layout_no_wrap(label.to_string(), font.clone(), fg);
                let br = egui::Rect::from_min_size(egui::pos2(lx + tw + 8.0, cy - g.size().y / 2.0 - 2.0), Vec2::new(g.size().x + 12.0, g.size().y + 4.0));
                p.rect_filled(br, CornerRadius::same(6), fill);
                p.galley(egui::pos2(br.left() + 6.0, br.top() + 2.0), g, fg);
                if tone == ClaudeTone::Running {
                    // A small travelling wave at the right end of the header.
                    let (n, w, sp) = (5usize, 2.5_f32, 3.0_f32);
                    let x_end = rect.right() - l.h_padding;
                    for i in 0..n {
                        let k = wave(i, n);
                        let h = 4.0 + 8.0 * k;
                        bar(x_end - (n - i) as f32 * (w + sp) + sp + w / 2.0, cy, w, h, ACCENT.gamma_multiply(0.3 + 0.7 * k), CornerRadius::same(2));
                    }
                }
            } else if note.is_some() {
                let (fill, fg, title) = if has_message { (Color32::from_rgb(0x3A, 0x22, 0x08), AMBER, "Info") } else { (Color32::from_rgb(0x0F, 0x2A, 0x1A), green_text, "Collé") };
                let c = egui::pos2(lx + 7.0, cy);
                p.circle_filled(c, 7.0, fill);
                if has_message {
                    let s = Stroke::new(1.6_f32, fg);
                    p.line_segment([c + Vec2::new(0.0, -3.4), c + Vec2::new(0.0, 0.8)], s);
                    p.circle_filled(c + Vec2::new(0.0, 3.2), 1.0, fg);
                } else {
                    let s = Stroke::new(1.6_f32, fg);
                    p.line_segment([c + Vec2::new(-3.2, 0.2), c + Vec2::new(-1.0, 2.5)], s);
                    p.line_segment([c + Vec2::new(-1.0, 2.5), c + Vec2::new(3.2, -2.4)], s);
                }
                p.text(egui::pos2(lx + 18.0, cy), egui::Align2::LEFT_CENTER, title, font, fg);
            }
            if let Some(b) = body {
                let g = p.layout(b, FontId::proportional(l.trans_font), body_color, l.preview_max_width);
                p.galley(egui::pos2(lx, rect.top() + head_h - l.v_padding / 2.0), g, body_color);
            }
        }
    }
}

impl App {
    /// The toast: a small dark capsule at the bottom centre of the window, above everything.
    fn show_toast(&mut self, ctx: &egui::Context, now: f64) {
        let Some((text, until, ok)) = self.toast.clone() else { return };
        if now >= until {
            self.toast = None;
            return;
        }
        egui::Area::new(egui::Id::new("toast"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -24.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgb(28, 31, 40))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(70, 76, 90)))
                    .corner_radius(CornerRadius::same(18))
                    .inner_margin(Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 4.0, if ok { GREEN } else { AMBER });
                            ui.label(RichText::new(text).size(13.5).color(TEXT));
                        });
                    });
            });
        if until.is_finite() {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(until - now));
        }
    }

    fn screenshot_step(&mut self, ctx: &egui::Context) {
        // --screenshot <file>: draw ~120 frames, save the picture, quit.
        if let Some(path) = self.screenshot.clone() {
            ctx.request_repaint();
            if self.frames >= 120 && !self.shot_requested {
                self.shot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let image = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = image {
                let [w, h] = img.size;
                let mut bytes = Vec::with_capacity(w * h * 4);
                for c in &img.pixels {
                    bytes.extend_from_slice(&c.to_array());
                }
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Err(e) = image::save_buffer(&path, &bytes, w as u32, h as u32, image::ColorType::Rgba8) {
                    eprintln!("screenshot: {e}");
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Transparent so the pill keeps see-through corners. The main window stays opaque:
        // `paint_background` fills all of it, title bar included (full-size content view).
        [0.0; 4]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        paint_background(ctx);

        if self.pill_test {
            let (levels, elapsed) = {
                let sh = self.shared.lock().unwrap();
                (sh.levels.iter().copied().collect::<Vec<f32>>(), sh.record_started.map(|t| t.elapsed().as_secs_f32()))
            };
            let now = ctx.input(|i| i.time);
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    // Two columns: bars on the left, the line wave on the right.
                    let done = "voix-fluide · terminé\nLe seuil de 0,4 seconde est TAP_HOLD_THRESHOLD dans engine.rs.";
                    let bars = PillStyle { wave: WaveStyle::Bars, timer: true };
                    let line = PillStyle { wave: WaveStyle::Line, timer: true };
                    let full = ui.max_rect();
                    for (column, rows) in [
                        [
                            (2u8, bars, Phase::Recording, None, None),
                            (2u8, bars, Phase::Idle, Some("Bonjour, ceci est un essai de dictée vocale. Le texte doit apparaître dans la fenêtre."), None),
                            (1u8, bars, Phase::Transcribing, None, None),
                        ],
                        [
                            (2u8, line, Phase::Recording, None, None),
                            (2u8, bars, Phase::Idle, None, Some((done, ClaudeTone::Done))),
                            (0u8, bars, Phase::Recording, None, None),
                        ],
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let cx = full.left() + full.width() * (0.25 + 0.5 * column as f32);
                        let mut y = full.top() + 24.0;
                        for (size, style, ph, note, claude) in rows {
                            let l = pill_layout(size);
                            let r = egui::Rect::from_min_size(egui::pos2(cx - l.overlay_width / 2.0, y), Vec2::new(l.overlay_width, l.overlay_height));
                            draw_pill(ui, r, l, style, false, ph, &levels, elapsed, note, false, claude, now);
                            y += l.overlay_height + 12.0;
                        }
                    }
                });
            self.screenshot_step(ctx);
            return;
        }
        // VF_PILL_DEMO=1 (test): pretend to dictate between 6 and 10 s after start.
        if std::env::var_os("VF_PILL_DEMO").is_some() {
            let t = ctx.input(|i| i.time);
            let mut sh = self.shared.lock().unwrap();
            let want = if (6.0..10.0).contains(&t) { Phase::Recording } else { Phase::Idle };
            if sh.phase != want {
                sh.phase = want;
                sh.record_started = Some(std::time::Instant::now());
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        // Closing the window keeps Voix Fluide running: the shortcut and the
        // floating pill keep working anywhere. Click the Dock icon to come back.
        if self.screenshot.is_none() && !self.quitting && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let now = ctx.input(|i| i.time);
        if now - self.ax_checked > AX_POLL_S || self.frames == 1 {
            self.ax_checked = now;
            let was = self.ax_trusted;
            self.ax_trusted = mac::accessibility_trusted();
            // FluidVoice logs "Accessibility permission granted" and ends its
            // permission flow when the 2 s poll sees access come back (0x100fcbb94).
            if !was && self.ax_trusted && self.frames > 1 && self.screenshot.is_none() {
                self.shared.lock().unwrap().message =
                    Some("Accessibilité autorisée : le texte sera collé automatiquement.".into());
            }
            if self.screenshot.is_none() {
                self.mic_access = mac::mic_access();
                self.devices = engine::input_devices();
                self.default_device = engine::default_input_name();
            }
        }

        let snapshot = {
            let mut sh = self.shared.lock().unwrap();
            sh.app_focused = focused;
            (
                sh.phase,
                sh.levels.iter().copied().collect::<Vec<f32>>(),
                sh.model.clone(),
                sh.last.clone(),
                sh.history.clone(),
                sh.message.clone(),
                sh.settings.clone(),
                sh.record_started.map(|t| t.elapsed().as_secs_f32()),
                sh.mic_dead,
                sh.last_wav.clone(),
                sh.target,
                sh.sessions.clone(),
            )
        };
        let (phase, levels, model, last, history, message, mut settings, elapsed, mic_dead, last_wav, target, sessions) = snapshot;
        let update_state = self.update.lock().unwrap().clone();
        let mic_blocked = matches!(self.mic_access, mac::MicAccess::Denied | mac::MicAccess::Restricted | mac::MicAccess::NotAsked);
        let before = settings.clone();

        // Menu bar: show the state, then apply the clicks (a mic choice goes through
        // `settings`, saved below like the window's combobox).
        if let Some(tray) = &mut self.tray {
            tray.refresh(phase, &model, settings.key.label, !last.is_empty(), &self.devices, settings.mic.as_deref());
            for action in tray.poll() {
                match action {
                    tray::TrayAction::CopyLast => mac::copy_text(&last),
                    tray::TrayAction::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    tray::TrayAction::SetMic(mic) => settings.mic = mic,
                    tray::TrayAction::Quit => {
                        self.quitting = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }

        let (status_txt, status_col) = match &model {
            ModelState::Ready(s) => (s.clone(), GREEN),
            ModelState::Loading => ("Chargement du modèle…".to_string(), MUTED),
            ModelState::Downloading(p) => (format!("Téléchargement {:.0} %", p * 100.0), ACCENT),
            ModelState::Missing => ("Aucun modèle".to_string(), MUTED),
            ModelState::Failed(_) => ("Erreur de modèle".to_string(), RED),
        };

        // Sidebar
        let nav = egui::SidePanel::left("nav")
            .exact_width(220.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(Color32::from_rgb(9, 13, 23)).inner_margin(Margin::symmetric(12, 18)))
            .show(ctx, |ui| {
                // Room for the traffic lights of the transparent title bar.
                ui.add_space(TITLEBAR_PT);
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.image(egui::load::SizedTexture::new(self.logo.id(), Vec2::splat(LOGO_PT)));
                    ui.label(RichText::new("Voix Fluide").size(17.0).strong().color(TEXT));
                });
                ui.add_space(14.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                caps(ui, "ACTIVITÉ");
                let running = sessions.iter().filter(|s| s.state == SessionState::Running).count();
                for (page, label) in [(Page::Home, "Accueil"), (Page::History, "Historique"), (Page::Claude, "Claude Code")] {
                    let badge = (page == Page::Claude && running > 0).then(|| running.to_string());
                    if nav_item(ui, label, self.page == page, badge.as_deref()).clicked() {
                        self.page = page;
                    }
                }
                ui.add_space(10.0);
                caps(ui, "RÉGLAGES");
                let update_available = matches!(update_state, UpdateState::Available { .. });
                for (page, label) in [(Page::Keys, "Raccourcis"), (Page::Model, "Modèle et langue"), (Page::Pill, "Pastille"), (Page::Audio, "Micro et collage"), (Page::Updates, "Mises à jour")] {
                    let resp = nav_item(ui, label, self.page == page, None);
                    if page == Page::Updates && update_available {
                        ui.painter().circle_filled(egui::pos2(resp.rect.right() - 16.0, resp.rect.center().y), 4.0, ACCENT);
                    }
                    if resp.clicked() {
                        self.page = page;
                    }
                }
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    egui::Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0_f32, PANEL_STROKE))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 4.0, status_col);
                                ui.label(RichText::new(&status_txt).size(12.0).color(SOFT));
                            });
                            ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(12.0).color(MUTED));
                        });
                });
            });
        let edge = nav.response.rect;
        ctx.layer_painter(egui::LayerId::background()).vline(edge.right(), edge.y_range(), Stroke::new(1.0_f32, Color32::from_rgb(27, 33, 48)));

        // Page
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(Margin { left: 32, right: 32, top: 26 + TITLEBAR_PT as i8, bottom: 26 }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.page {
                        Page::Home => {
                            page_header(ui, "Accueil", "Parlez, le texte s'écrit là où se trouve votre curseur. Tout reste sur ce Mac.");

                            if mic_blocked || mic_dead {
                                let why = match self.mic_access {
                                    mac::MicAccess::NotAsked => "macOS n'a pas encore donné l'accès au micro. Sans cet accès il n'envoie que du silence, et la transcription invente des mots.",
                                    mac::MicAccess::Denied | mac::MicAccess::Restricted => "L'accès au micro est refusé dans les réglages de macOS. Activez Voix Fluide dans Confidentialité et sécurité > Micro.",
                                    mac::MicAccess::Allowed => "Le dernier enregistrement ne contenait aucun son. Vérifiez le micro choisi dans les réglages.",
                                };
                                let not_asked = self.mic_access == mac::MicAccess::NotAsked;
                                warn_card(ui, "Le micro est inaccessible", why, |ui| {
                                    if not_asked && ui.add(egui::Button::new(RichText::new("Autoriser le micro").color(Color32::WHITE)).fill(ACCENT)).clicked() {
                                        mac::request_mic_access();
                                    }
                                    if ui.button("Ouvrir les réglages du micro").clicked() {
                                        mac::open_microphone_settings();
                                    }
                                });
                            }

                            if settings.paste && !self.ax_trusted && self.screenshot.is_none() {
                                warn_card(
                                    ui,
                                    "Le collage automatique n'est pas autorisé",
                                    "macOS ne reconnaît pas cette version de Voix Fluide dans Accessibilité. Si l'interrupteur est déjà activé, il vise une ancienne version : retirez-la avec « – », puis cliquez « Réautoriser » et activez Voix Fluide. Le texte reste copié dans le presse-papiers en attendant.",
                                    |ui| {
                                        if ui.add(egui::Button::new(RichText::new("Réautoriser").color(Color32::WHITE)).fill(ACCENT)).clicked() {
                                            mac::request_accessibility();
                                            mac::open_accessibility_settings();
                                        }
                                        if ui.button("Ouvrir les réglages d'accessibilité").clicked() {
                                            mac::open_accessibility_settings();
                                        }
                                    },
                                );
                            }

                            // Recorder card
                            card(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    let w = ui.available_width().min(420.0);
                                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 64.0), Sense::hover());
                                    let p = ui.painter();
                                    let n = levels.len();
                                    let gap = 5.0;
                                    let bw = ((w - gap * (n as f32 - 1.0)) / n as f32).max(2.0);
                                    for (i, l) in levels.iter().enumerate() {
                                        let lvl = match phase {
                                            Phase::Recording => *l,
                                            Phase::Transcribing => 0.15 + 0.12 * ((now as f32 * 6.0 + i as f32 * 0.45).sin() + 1.0),
                                            Phase::Idle => 0.0,
                                        };
                                        let h = 8.0 + lvl * 52.0;
                                        let x = rect.left() + i as f32 * (bw + gap);
                                        let r = egui::Rect::from_center_size(egui::pos2(x + bw / 2.0, rect.center().y), Vec2::new(bw, h));
                                        let col = match phase {
                                            Phase::Recording => Color32::WHITE,
                                            Phase::Transcribing => ACCENT,
                                            Phase::Idle => Color32::from_rgb(60, 64, 74),
                                        };
                                        p.rect_filled(r, CornerRadius::same((bw / 2.0).min(6.0) as u8), col);
                                    }
                                    ui.add_space(12.0);

                                    let (label, fill) = match phase {
                                        Phase::Idle => ("Commencer la dictée", ACCENT),
                                        Phase::Recording => ("Arrêter et transcrire", RED),
                                        Phase::Transcribing => ("Transcription…", Color32::from_rgb(44, 48, 60)),
                                    };
                                    let btn = egui::Button::new(RichText::new(label).size(16.0).strong().color(Color32::WHITE))
                                        .fill(fill)
                                        .corner_radius(CornerRadius::same(22))
                                        .min_size(Vec2::new(260.0, 44.0));
                                    if ui.add_enabled(phase != Phase::Transcribing, btn).clicked() {
                                        self.send(Cmd::Toggle);
                                    }
                                    ui.add_space(2.0);
                                    match phase {
                                        Phase::Idle => {
                                            let verb = match settings.mode {
                                                Mode::Toggle => "Appuyez sur",
                                                Mode::Hold => "Maintenez",
                                                Mode::Automatic => "Appuyez ou maintenez",
                                            };
                                            key_line(ui, &[verb, &short_key(settings.key.label), "pour dicter"]);
                                            key_line(ui, &["Maintenez", &short_key(settings.session_key.label), "pour demander à Claude Code"]);
                                        }
                                        Phase::Recording => {
                                            ui.label(RichText::new(format!("Écoute… {:.1} s", elapsed.unwrap_or(0.0))).color(MUTED));
                                        }
                                        Phase::Transcribing => {
                                            ui.label(RichText::new("Transcription en cours sur cet ordinateur…").color(MUTED));
                                        }
                                    }
                                    if phase == Phase::Recording && ui.small_button("Annuler").clicked() {
                                        self.send(Cmd::Cancel);
                                    }
                                    if let Some(m) = &message {
                                        ui.label(RichText::new(m).color(AMBER));
                                    }
                                });
                            });
                            ui.add_space(12.0);

                            // Last transcription
                            card(ui, |ui| {
                                ui.horizontal(|ui| {
                                    caps(ui, "DERNIÈRE TRANSCRIPTION");
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.add_enabled_ui(!last.is_empty(), |ui| secondary_button(ui, "Copier")).inner.clicked() {
                                            mac::copy_text(&last);
                                        }
                                    });
                                });
                                let shown = if last.is_empty() {
                                    RichText::new("Votre texte apparaîtra ici.").color(MUTED).italics()
                                } else {
                                    RichText::new(&last).size(15.0).color(TEXT)
                                };
                                ui.add(egui::Label::new(shown).wrap().selectable(true));
                            });
                        }

                        Page::History => {
                            page_header(ui, "Historique", "Vos 50 dernières dictées, gardées sur ce Mac.");
                            card(ui, |ui| {
                                if history.is_empty() {
                                    ui.label(RichText::new("Aucune dictée pour l'instant.").color(MUTED).italics());
                                }
                                for (i, e) in history.iter().enumerate().take(20) {
                                    if i > 0 {
                                        row_separator(ui);
                                    }
                                    ui.push_id(i, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("{} · {:.0} s", e.when, e.seconds)).size(12.0).color(MUTED));
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                if secondary_button(ui, "Copier").on_hover_text("Copier ce texte").clicked() {
                                                    mac::copy_text(&e.text);
                                                }
                                            });
                                        });
                                        ui.add(egui::Label::new(RichText::new(&e.text).color(TEXT)).wrap().selectable(true));
                                    });
                                }
                            });
                        }

                        Page::Claude => {
                            page_header(ui, "Claude Code", "Posez une question à voix haute en nommant le dépôt : Claude Code lit le code sans rien modifier.");
                            card(ui, |ui| {
                                caps(ui, "MINI-SESSIONS");
                                if sessions.is_empty() {
                                    key_line(ui, &["Maintenez", &short_key(settings.session_key.label), "et dites : « Dans Fourchette, … »"]);
                                }
                                for s in sessions.iter().rev().take(20) {
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);
                                    ui.push_id(s.id, |ui| {
                                        let (state, fg, bg) = match &s.state {
                                            SessionState::Running => (format!("en cours · {:.0} s", s.started.elapsed().as_secs_f32()), Color32::from_rgb(0x6F, 0xA8, 0xFF), Color32::from_rgb(0x0D, 0x22, 0x44)),
                                            SessionState::Done { .. } => ("terminé".to_string(), Color32::from_rgb(0x5B, 0xD9, 0x8A), Color32::from_rgb(0x0F, 0x2A, 0x1A)),
                                            SessionState::Failed(_) => ("erreur".to_string(), AMBER, Color32::from_rgb(0x3A, 0x22, 0x08)),
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("{} · {}", s.when, s.repo_name())).size(12.0).color(MUTED));
                                            egui::Frame::new()
                                                .fill(bg)
                                                .corner_radius(CornerRadius::same(6))
                                                .inner_margin(Margin::symmetric(7, 2))
                                                .show(ui, |ui| {
                                                    ui.label(RichText::new(state).size(11.0).color(fg));
                                                });
                                            if let SessionState::Done { session_id, .. } = &s.state {
                                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                    let cmd = claude::resume_command(&s.repo, session_id);
                                                    if secondary_button(ui, "Reprendre au terminal").on_hover_text(format!("Ouvre le Terminal et lance :\n{cmd}\n(la commande est aussi copiée)")).clicked() {
                                                        mac::copy_text(&cmd);
                                                        self.toast = Some(match claude::open_in_terminal(&s.repo, session_id) {
                                                            Ok(()) => ("Commande copiée · Terminal ouvert".into(), now + TOAST_S, true),
                                                            Err(e) => {
                                                                eprintln!("terminal: {e}");
                                                                ("Commande copiée — collez-la dans un terminal".into(), now + TOAST_S, false)
                                                            }
                                                        });
                                                    }
                                                });
                                            }
                                        });
                                        ui.add(egui::Label::new(RichText::new(&s.question).color(SOFT)).wrap());
                                        match &s.state {
                                            SessionState::Running => {}
                                            SessionState::Done { resume, reponse, .. } => {
                                                ui.add(egui::Label::new(RichText::new(resume).strong().color(TEXT)).wrap().selectable(true));
                                                ui.add(egui::Label::new(RichText::new(reponse).color(TEXT)).wrap().selectable(true));
                                            }
                                            SessionState::Failed(e) => {
                                                ui.add(egui::Label::new(RichText::new(e).color(AMBER)).wrap().selectable(true));
                                            }
                                        }
                                    });
                                }
                            });
                            ui.add_space(12.0);
                            card(ui, |ui| {
                                setting_row(ui, "Dossier des dépôts", "Voix Fluide cherche le dépôt nommé dans la phrase.", |ui| {
                                    ui.add(egui::TextEdit::singleline(&mut settings.repo_root).desired_width(200.0));
                                });
                                row_separator(ui);
                                setting_row(ui, "Lire le résumé à voix haute", "", |ui| {
                                    switch(ui, &mut settings.speak);
                                });
                            });
                        }

                        Page::Keys => {
                            page_header(ui, "Raccourcis", "Deux touches distinctes : une pour dicter, une pour interroger Claude Code.");
                            card(ui, |ui| {
                                setting_row(ui, "Dictée", "La touche qui enregistre, transcrit et colle le texte.", |ui| {
                                    egui::ComboBox::from_id_salt("key").width(200.0).selected_text(settings.key.label).show_ui(ui, |ui| {
                                        for k in mac::SHORTCUTS {
                                            // The mini-session key can't also dictate.
                                            ui.add_enabled_ui(k != settings.session_key, |ui| ui.selectable_value(&mut settings.key, k, k.label));
                                        }
                                    });
                                });
                                row_separator(ui);
                                setting_row(ui, "Mini-session Claude Code", "Elle doit être différente de celle de la dictée.", |ui| {
                                    egui::ComboBox::from_id_salt("session_key").width(200.0).selected_text(settings.session_key.label).show_ui(ui, |ui| {
                                        for k in mac::SHORTCUTS {
                                            ui.add_enabled_ui(k != settings.key, |ui| ui.selectable_value(&mut settings.session_key, k, k.label));
                                        }
                                    });
                                });
                            });
                            ui.add_space(12.0);
                            card(ui, |ui| {
                                ui.label(RichText::new("Mode de la dictée").color(TEXT));
                                ui.label(RichText::new("Comment la touche déclenche l'enregistrement.").size(12.0).color(MUTED));
                                ui.add_space(4.0);
                                let w = (ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0;
                                ui.horizontal(|ui| {
                                    for m in Mode::ALL {
                                        let title = match m {
                                            Mode::Automatic => "Automatique",
                                            _ => m.label(),
                                        };
                                        if choice_card(ui, w, settings.mode == m, title, m.help()).clicked() {
                                            settings.mode = m;
                                        }
                                    }
                                });
                            });
                        }

                        Page::Model => {
                            page_header(ui, "Modèle et langue", "La transcription tourne entièrement sur ce Mac. Téléchargement unique par modèle.");
                            card(ui, |ui| {
                                setting_row(ui, "Langue de dictée", "", |ui| {
                                    let lang_name = LANGUAGES.iter().find(|(c, _)| *c == settings.language).map(|(_, n)| *n).unwrap_or("Détection automatique");
                                    egui::ComboBox::from_id_salt("lang").width(200.0).selected_text(lang_name).show_ui(ui, |ui| {
                                        for (c, n) in LANGUAGES {
                                            ui.selectable_value(&mut settings.language, c.to_string(), n);
                                        }
                                    });
                                });
                            });
                            ui.add_space(12.0);
                            caps(ui, "MODÈLE ACTIF");
                            egui::Frame::new()
                                .fill(PANEL)
                                .stroke(Stroke::new(1.0_f32, ACCENT))
                                .corner_radius(CornerRadius::same(14))
                                .inner_margin(Margin::same(18))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 5.0, status_col);
                                        ui.label(RichText::new(settings.model.label()).size(15.0).strong().color(TEXT));
                                    });
                                    ui.label(RichText::new(settings.model.about()).size(12.0).color(MUTED));
                                    match &model {
                                        ModelState::Downloading(p) => {
                                            ui.add(egui::ProgressBar::new(*p).desired_width(300.0).show_percentage());
                                        }
                                        ModelState::Missing => {
                                            ui.label(RichText::new("Pas encore téléchargé : fonctionne ensuite entièrement sur cet ordinateur.").size(12.0).color(MUTED));
                                            if ui.add(egui::Button::new(RichText::new("Télécharger le modèle").color(Color32::WHITE)).fill(ACCENT)).clicked() {
                                                self.job(Job::Download(settings.model));
                                            }
                                        }
                                        ModelState::Failed(e) => {
                                            ui.label(RichText::new(e).size(12.0).color(RED));
                                            if secondary_button(ui, "Réessayer").clicked() {
                                                self.job(Job::Download(settings.model));
                                            }
                                        }
                                        _ => {}
                                    }
                                });
                            ui.add_space(12.0);
                            caps(ui, "AUTRES MODÈLES");
                            card(ui, |ui| {
                                let mut first = true;
                                for m in ModelSize::ALL {
                                    if m == settings.model {
                                        continue;
                                    }
                                    if !first {
                                        row_separator(ui);
                                    }
                                    first = false;
                                    setting_row(ui, m.label(), m.about(), |ui| {
                                        let downloaded = whisper::is_downloaded(m);
                                        if secondary_button(ui, if downloaded { "Activer" } else { "Choisir" }).clicked() {
                                            settings.model = m;
                                        }
                                        if !downloaded {
                                            ui.label(RichText::new("Non téléchargé").size(12.0).color(MUTED));
                                        }
                                    });
                                }
                            });
                        }

                        Page::Pill => {
                            page_header(ui, "Pastille", "L'indicateur qui flotte au bord de l'écran pendant la dictée.");
                            caps(ui, "APERÇU");
                            card(ui, |ui| {
                                if !settings.overlay {
                                    ui.multiply_opacity(0.35);
                                }
                                let l = pill_layout(settings.overlay_size);
                                let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), l.overlay_height), Sense::hover());
                                let canvas = egui::Rect::from_center_size(area.center(), Vec2::new(l.overlay_width.min(area.width()), l.overlay_height));
                                // Simulated states and wave: no microphone, worker or `Shared` involved.
                                let (ph, note, claude) = match (now / PREVIEW_STEP_S) as u64 % 4 {
                                    0 => (Phase::Recording, None, None),
                                    1 => (Phase::Transcribing, None, None),
                                    2 => (Phase::Idle, Some(if settings.overlay_text { PREVIEW_TEXT } else { "" }), None),
                                    _ => (Phase::Idle, None, Some((PREVIEW_CLAUDE, ClaudeTone::Done))),
                                };
                                let levels: Vec<f32> = (0..engine::LEVEL_BARS)
                                    .map(|i| {
                                        let t = now as f32;
                                        (0.5 + 0.4 * (t * 5.0 + i as f32 * 0.8).sin() * (t * 1.7 + i as f32 * 0.3).cos()).clamp(0.0, 1.0)
                                    })
                                    .collect();
                                let elapsed = Some((now % PREVIEW_STEP_S) as f32 + 4.0);
                                draw_pill(ui, canvas, l, PillStyle::of(&settings), settings.overlay_top, ph, &levels, elapsed, note, false, claude, now);
                            });
                            ui.add_space(12.0);
                            caps(ui, "RÉGLAGES");
                            card(ui, |ui| {
                                setting_row(ui, "Afficher la pastille", "Un indicateur pendant la dictée, où que vous soyez.", |ui| {
                                    switch(ui, &mut settings.overlay);
                                });
                                ui.add_enabled_ui(settings.overlay, |ui| {
                                    row_separator(ui);
                                    // The controls run right to left: add the choices in reverse.
                                    setting_row(ui, "Position", "", |ui| {
                                        ui.selectable_value(&mut settings.overlay_top, true, "En haut de l'écran");
                                        ui.selectable_value(&mut settings.overlay_top, false, "En bas de l'écran");
                                    });
                                    row_separator(ui);
                                    setting_row(ui, "Taille", "", |ui| {
                                        for (i, name) in PILL_SIZE_NAMES.iter().enumerate().rev() {
                                            ui.selectable_value(&mut settings.overlay_size, i as u8, *name);
                                        }
                                    });
                                    row_separator(ui);
                                    setting_row(ui, "Écran", "Choisi au début de chaque dictée.", |ui| {
                                        egui::ComboBox::from_id_salt("pill_screen").width(200.0).selected_text(settings.overlay_screen.label()).show_ui(ui, |ui| {
                                            for o in OverlayScreen::ALL {
                                                ui.selectable_value(&mut settings.overlay_screen, o, o.label());
                                            }
                                        });
                                    });
                                    row_separator(ui);
                                    setting_row(ui, "Onde", "", |ui| {
                                        for w in WaveStyle::ALL.into_iter().rev() {
                                            ui.selectable_value(&mut settings.overlay_wave, w, w.label());
                                        }
                                    });
                                    row_separator(ui);
                                    setting_row(ui, "Chrono", "La durée de l'écoute, à droite de la pastille.", |ui| {
                                        switch(ui, &mut settings.overlay_timer);
                                    });
                                    row_separator(ui);
                                    setting_row(ui, "Montrer le texte collé", "Sous la pastille, une fois la dictée finie. Les erreurs s'affichent toujours.", |ui| {
                                        switch(ui, &mut settings.overlay_text);
                                    });
                                });
                            });
                        }

                        Page::Audio => {
                            page_header(ui, "Micro et collage", "Quel micro écoute, et ce qui arrive une fois le texte transcrit.");
                            caps(ui, "MICRO");
                            card(ui, |ui| {
                                let default_label = match &self.default_device {
                                    Some(d) => format!("Par défaut du système ({d})"),
                                    None => "Par défaut du système".to_string(),
                                };
                                let current = settings.mic.clone().or_else(|| self.default_device.clone()).unwrap_or_default();
                                let help = if engine::looks_bluetooth(&current) {
                                    "Un micro Bluetooth passe le casque en mode « appel » et dégrade le son. Préférez un micro filaire, USB ou celui de l'écran."
                                } else {
                                    "Le périphérique qui écoute pendant la dictée."
                                };
                                setting_row(ui, "Périphérique d'entrée", help, |ui| {
                                    let shown = settings.mic.clone().unwrap_or_else(|| default_label.clone());
                                    egui::ComboBox::from_id_salt("mic").width(200.0).selected_text(shown).show_ui(ui, |ui| {
                                        ui.selectable_value(&mut settings.mic, None, default_label.clone());
                                        for d in &self.devices {
                                            ui.selectable_value(&mut settings.mic, Some(d.clone()), d);
                                        }
                                    });
                                });
                                row_separator(ui);
                                setting_row(ui, "Ignorer les enregistrements silencieux", "Moins de 4 secondes sans aucun son.", |ui| {
                                    switch(ui, &mut settings.skip_silent);
                                });
                                row_separator(ui);
                                setting_row(ui, "Dernière dictée", "Réécoutez l'enregistrement pour vérifier le micro.", |ui| {
                                    if ui.add_enabled_ui(last_wav.is_some(), |ui| secondary_button(ui, "Réécouter")).inner.clicked() {
                                        if let Some(p) = &last_wav {
                                            let _ = std::process::Command::new("open").arg(p).spawn();
                                        }
                                    }
                                });
                            });
                            ui.add_space(12.0);
                            caps(ui, "COLLAGE");
                            card(ui, |ui| {
                                setting_row(ui, "Coller dans l'application active", "", |ui| {
                                    switch(ui, &mut settings.paste);
                                });
                                row_separator(ui);
                                setting_row(ui, "Garder aussi le texte dans le presse-papiers", "", |ui| {
                                    switch(ui, &mut settings.keep_clipboard);
                                });
                                row_separator(ui);
                                setting_row(ui, "Sons de début et de fin", "", |ui| {
                                    switch(ui, &mut settings.sounds);
                                });
                            });
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(RichText::new("Fermer la fenêtre garde Voix Fluide actif : appuyez sur le raccourci depuis n'importe quelle app. Cliquez sur l'icône du Dock pour revenir ici.").size(12.0).color(MUTED)).wrap());
                            });
                            if secondary_button(ui, "Quitter Voix Fluide").clicked() {
                                self.quitting = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }

                        Page::Updates => {
                            page_header(ui, "Mises à jour", "Voix Fluide cherche la dernière version publiée sur GitHub et l'installe sur ce Mac.");
                            caps(ui, "VERSION INSTALLÉE");
                            card(ui, |ui| {
                                let commit = env!("VF_COMMIT");
                                let kind = if env!("VF_RELEASE") == "1" { "release" } else { "build locale" };
                                let help = format!("Commit {} · {kind}", &commit[..commit.len().min(7)]);
                                setting_row(ui, &format!("Version {}", env!("CARGO_PKG_VERSION")), &help, |ui| {
                                    let busy = matches!(update_state, UpdateState::Checking | UpdateState::Downloading);
                                    if ui.add_enabled_ui(!busy, |ui| secondary_button(ui, "Rechercher")).inner.clicked() {
                                        update::spawn_check(self.update.clone(), ui.ctx().clone());
                                    }
                                });
                            });
                            ui.add_space(12.0);
                            caps(ui, "DERNIÈRE VERSION");
                            card(ui, |ui| {
                                let (line, col) = match &update_state {
                                    UpdateState::Idle => ("Pas encore vérifié.".to_string(), MUTED),
                                    UpdateState::Checking => ("Recherche…".to_string(), MUTED),
                                    UpdateState::UpToDate => ("À jour".to_string(), GREEN),
                                    UpdateState::Available { version, commit, .. } => (format!("Version {version} disponible (commit {})", &commit[..commit.len().min(7)]), ACCENT),
                                    UpdateState::Downloading => ("Téléchargement…".to_string(), ACCENT),
                                    UpdateState::Failed(e) => (e.clone(), AMBER),
                                };
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                    ui.painter().circle_filled(r.center(), 5.0, col);
                                    ui.add(egui::Label::new(RichText::new(line).size(15.0).strong().color(TEXT)).wrap());
                                });
                                if let UpdateState::Available { version, commit, sha256 } = &update_state {
                                    ui.label(RichText::new("Le fichier est vérifié (SHA-256) avant de remplacer cette version, puis Voix Fluide se relance.").size(12.0).color(MUTED));
                                    ui.add_space(4.0);
                                    let running = sessions.iter().filter(|s| s.state == SessionState::Running).count();
                                    let blocked = update::install_blocked(phase, running);
                                    let btn = egui::Button::new(RichText::new("Installer et relancer").color(Color32::WHITE)).fill(ACCENT);
                                    if ui.add_enabled(blocked.is_none(), btn).clicked() {
                                        let release = update::Release { version: version.clone(), commit: commit.clone(), sha256: sha256.clone() };
                                        update::spawn_install(self.update.clone(), release, self.shared.clone(), ui.ctx().clone());
                                    }
                                    if let Some(why) = blocked {
                                        ui.label(RichText::new(why).size(12.0).color(AMBER));
                                    }
                                }
                                if update_state == UpdateState::Downloading {
                                    ui.label(RichText::new("Voix Fluide se relancera une fois le fichier vérifié.").size(12.0).color(MUTED));
                                }
                            });
                        }
                    }
                });
            });

        self.overlay(ctx, &settings, phase, target, &levels, elapsed, &last, &message, &sessions, now);
        self.show_toast(ctx, now);

        if settings.model != before.model {
            let mut sh = self.shared.lock().unwrap();
            sh.model = if whisper::is_downloaded(settings.model) {
                ModelState::Loading
            } else {
                ModelState::Missing
            };
            drop(sh);
            if whisper::is_downloaded(settings.model) {
                self.job(Job::Load(settings.model));
            }
        }
        if settings.mode != before.mode
            || settings.key != before.key
            || settings.language != before.language
            || settings.model != before.model
            || settings.paste != before.paste
            || settings.keep_clipboard != before.keep_clipboard
            || settings.sounds != before.sounds
            || settings.mic != before.mic
            || settings.skip_silent != before.skip_silent
            || settings.overlay != before.overlay
            || settings.overlay_top != before.overlay_top
            || settings.overlay_size != before.overlay_size
            || settings.overlay_timer != before.overlay_timer
            || settings.overlay_wave != before.overlay_wave
            || settings.overlay_text != before.overlay_text
            || settings.overlay_screen != before.overlay_screen
            || settings.session_key != before.session_key
            || settings.repo_root != before.repo_root
            || settings.speak != before.speak
        {
            settings.save();
            self.shared.lock().unwrap().settings = settings;
        }

        if phase != Phase::Idle {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        } else if self.page == Page::Pill {
            // The preview on the Pastille page moves on its own.
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        } else if sessions.iter().any(|s| s.state == SessionState::Running) {
            // Keeps the elapsed time of running mini-sessions moving.
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        self.screenshot_step(ctx);
    }
}

/// The `--version` line: `voix-fluide 0.3.0 (f0a67a9c1d2e, release)`.
fn version_line(version: &str, commit: &str, release: bool) -> String {
    let short = &commit[..commit.len().min(12)];
    let kind = if release { "release" } else { "build locale" };
    format!("voix-fluide {version} ({short}, {kind})")
}

#[cfg(test)]
mod tests {
    use super::version_line;

    #[test]
    fn version_line_shortens_the_commit() {
        let sha = "f0a67a9c1d2e3f405162738495a6b7c8d9e0f1a2";
        assert_eq!(version_line("0.3.0", sha, true), "voix-fluide 0.3.0 (f0a67a9c1d2e, release)");
        assert_eq!(version_line("0.3.0", "unknown", false), "voix-fluide 0.3.0 (unknown, build locale)");
    }
}
