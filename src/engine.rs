//! Recording, shortcut handling and the transcription worker.
//! Rules come from MECHANICS.md (tap/hold threshold, minimum length, modes).

use crate::mac::{self, ShortcutKey};
use crate::whisper::{self, Engine, ModelSize};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// GlobalHotkeyManager.automaticTapThresholdSeconds (0x10069f624).
pub const TAP_HOLD_THRESHOLD: f64 = 0.4;
/// Whisper refuses fewer samples than this (WhisperProvider.transcribe, 0x100802778).
pub const MIN_SAMPLES: usize = 16000;
pub const SAMPLE_RATE: u32 = 16000;
pub const LEVEL_BARS: usize = 28;
/// AudioCaptureIdlePolicy.SilentPCMRecoveryWatchdog.maximumRMS (0x10056f890): 1e-5.
pub const DEAD_MIC_MAX_RMS: f32 = 1e-5;
/// AudioCaptureIdlePolicy.SilentPCMRecoveryWatchdog.maximumPeak (0x10056f8ac): 5e-5.
pub const DEAD_MIC_MAX_PEAK: f32 = 5e-5;
/// ASRService.assessShortAudioSilence (0x10054e294): 20 ms frames (rate / 50) and
/// skip when RMS < 0.002, peak < 0.01 and loudest frame RMS < 0.0045.
pub const SILENCE_MAX_RMS: f32 = 0.002;
pub const SILENCE_MAX_PEAK: f32 = 0.01;
pub const SILENCE_MAX_FRAME_RMS: f32 = 0.0045;
/// Setting text: "a recording up to four seconds" (estimated as 4000 ms).
pub const SILENCE_MAX_MS: u64 = 4000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Toggle,
    Hold,
    Automatic,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Toggle, Mode::Hold, Mode::Automatic];
    pub fn label(self) -> &'static str {
        match self {
            Mode::Toggle => "Bascule",
            Mode::Hold => "Maintien",
            Mode::Automatic => "Automatique (les deux)",
        }
    }
    pub fn help(self) -> &'static str {
        match self {
            Mode::Toggle => "Appuyez une fois pour commencer, encore une fois pour arrêter.",
            Mode::Hold => "Enregistre seulement tant que la touche est maintenue.",
            Mode::Automatic => "Appui court pour basculer, appui long pour parler en maintenant.",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Mode::Toggle => "toggle",
            Mode::Hold => "hold",
            Mode::Automatic => "automatic",
        }
    }
    pub fn from_id(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.id() == s)
    }
}

pub const LANGUAGES: [(&str, &str); 8] = [
    ("auto", "Détection automatique"),
    ("fr", "Français"),
    ("en", "English"),
    ("es", "Español"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("nl", "Nederlands"),
];

#[derive(Clone, Debug)]
pub struct Settings {
    pub mode: Mode,
    pub key: ShortcutKey,
    pub language: String,
    pub model: ModelSize,
    pub paste: bool,
    pub keep_clipboard: bool,
    pub sounds: bool,
    /// Input device name; None = the system default (FluidVoice: preferredMicrophoneUID).
    pub mic: Option<String>,
    /// FluidVoice "SkipSilentRecordingsEnabled", off by default.
    pub skip_silent: bool,
    /// Floating pill shown anywhere while dictating (FluidVoice's BottomOverlayPanel).
    pub overlay: bool,
    /// FluidVoice OverlayPosition: "Top of Screen" / "Bottom of Screen" (default bottom).
    pub overlay_top: bool,
    /// FluidVoice OverlaySize: 0 Pill, 1 Small, 2 Medium (default), 3 Large.
    pub overlay_size: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Toggle,
            key: mac::SHORTCUTS[0],
            language: "auto".into(),
            model: ModelSize::Base,
            paste: true,
            keep_clipboard: false,
            sounds: true,
            mic: None,
            skip_silent: false,
            overlay: true,
            overlay_top: false,
            overlay_size: 2,
        }
    }
}

impl Settings {
    fn path() -> std::path::PathBuf {
        whisper::app_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(Self::path()) else {
            return s;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            return s;
        };
        if let Some(m) = v["mode"].as_str().and_then(Mode::from_id) {
            s.mode = m;
        }
        if let Some(code) = v["key"].as_u64() {
            if let Some(k) = mac::SHORTCUTS.iter().find(|k| k.code as u64 == code) {
                s.key = *k;
            }
        }
        if let Some(l) = v["language"].as_str() {
            s.language = l.to_string();
        }
        if let Some(m) = v["model"].as_str().and_then(ModelSize::from_id) {
            s.model = m;
        }
        if let Some(b) = v["paste"].as_bool() {
            s.paste = b;
        }
        if let Some(b) = v["keep_clipboard"].as_bool() {
            s.keep_clipboard = b;
        }
        if let Some(b) = v["sounds"].as_bool() {
            s.sounds = b;
        }
        s.mic = v["mic"].as_str().map(str::to_string);
        if let Some(b) = v["skip_silent"].as_bool() {
            s.skip_silent = b;
        }
        if let Some(b) = v["overlay"].as_bool() {
            s.overlay = b;
        }
        if let Some(b) = v["overlay_top"].as_bool() {
            s.overlay_top = b;
        }
        if let Some(n) = v["overlay_size"].as_u64() {
            s.overlay_size = n.min(3) as u8;
        }
        s
    }

    pub fn save(&self) {
        let v = serde_json::json!({
            "mode": self.mode.id(),
            "key": self.key.code,
            "language": self.language,
            "model": self.model.id(),
            "paste": self.paste,
            "keep_clipboard": self.keep_clipboard,
            "sounds": self.sounds,
            "mic": self.mic,
            "skip_silent": self.skip_silent,
            "overlay": self.overlay,
            "overlay_top": self.overlay_top,
            "overlay_size": self.overlay_size,
        });
        let _ = std::fs::create_dir_all(whisper::app_dir());
        let _ = std::fs::write(Self::path(), serde_json::to_string_pretty(&v).unwrap_or_default());
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelState {
    Missing,
    Downloading(f32),
    Loading,
    Ready(String),
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub text: String,
    pub when: String,
    pub seconds: f32,
}

pub struct Shared {
    pub phase: Phase,
    pub levels: VecDeque<f32>,
    pub model: ModelState,
    pub last: String,
    pub history: Vec<Entry>,
    pub message: Option<String>,
    pub settings: Settings,
    pub app_focused: bool,
    pub record_started: Option<Instant>,
    /// The last recording gave only exact silence (no microphone access).
    pub mic_dead: bool,
    pub last_wav: Option<std::path::PathBuf>,
}

pub type SharedRef = Arc<Mutex<Shared>>;

pub enum Cmd {
    KeyDown,
    KeyUp,
    Toggle,
    Cancel,
}

pub enum Job {
    Load(ModelSize),
    Download(ModelSize),
    Transcribe(Vec<f32>, f32),
}

pub struct Handles {
    pub cmd: Sender<Cmd>,
    pub job: Sender<Job>,
}

pub fn new_shared(settings: Settings) -> SharedRef {
    let model = if whisper::is_downloaded(settings.model) {
        ModelState::Loading
    } else {
        ModelState::Missing
    };
    Arc::new(Mutex::new(Shared {
        phase: Phase::Idle,
        levels: VecDeque::from(vec![0.0; LEVEL_BARS]),
        model,
        last: String::new(),
        history: Vec::new(),
        message: None,
        settings,
        app_focused: true,
        record_started: None,
        mic_dead: false,
        last_wav: None,
    }))
}

/// Start the shortcut watcher, the recorder and the transcription worker.
pub fn start(shared: SharedRef, ctx: egui::Context) -> Handles {
    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    let (job_tx, job_rx) = channel::<Job>();

    {
        let shared = shared.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || worker(job_rx, shared, ctx));
    }
    {
        let shared = shared.clone();
        let ctx = ctx.clone();
        let job_tx = job_tx.clone();
        std::thread::spawn(move || controller(cmd_rx, job_tx, shared, ctx));
    }
    {
        let shared = shared.clone();
        let cmd_tx = cmd_tx.clone();
        std::thread::spawn(move || {
            let mut was_down = false;
            loop {
                let key = shared.lock().unwrap().settings.key;
                let down = mac::key_down(key);
                if down != was_down {
                    let _ = cmd_tx.send(if down { Cmd::KeyDown } else { Cmd::KeyUp });
                    was_down = down;
                }
                std::thread::sleep(Duration::from_millis(8));
            }
        });
    }

    let model = shared.lock().unwrap().settings.model;
    if whisper::is_downloaded(model) {
        let _ = job_tx.send(Job::Load(model));
    }
    Handles {
        cmd: cmd_tx,
        job: job_tx,
    }
}

struct Recorder {
    _stream: cpal::Stream,
    buf: Arc<Mutex<Vec<f32>>>,
    rate: u32,
}

fn open_mic(shared: SharedRef, ctx: egui::Context) -> Result<Recorder, String> {
    let host = cpal::default_host();
    let wanted = shared.lock().unwrap().settings.mic.clone();
    let chosen = wanted.and_then(|name| {
        host.input_devices()
            .ok()?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
    });
    let dev = match chosen {
        Some(d) => d,
        None => host.default_input_device().ok_or("Aucun micro trouvé")?,
    };
    let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
    let channels = cfg.channels() as usize;
    let rate = cfg.sample_rate().0;
    let buf = Arc::new(Mutex::new(Vec::<f32>::with_capacity(rate as usize * 30)));
    let b2 = buf.clone();
    let err = |e| eprintln!("micro: {e}");
    let push = move |mono: &mut dyn Iterator<Item = f32>| {
        let mut sum = 0.0f32;
        let mut n = 0usize;
        let mut b = b2.lock().unwrap();
        for s in mono {
            b.push(s);
            sum += s * s;
            n += 1;
        }
        drop(b);
        if n > 0 {
            let rms = (sum / n as f32).sqrt();
            let level = (rms * 9.0).min(1.0);
            let mut sh = shared.lock().unwrap();
            sh.levels.pop_front();
            sh.levels.push_back(level);
            drop(sh);
            ctx.request_repaint();
        }
    };
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => {
            let push = push;
            dev.build_input_stream(
                &cfg.into(),
                move |data: &[f32], _| {
                    let mut it = data
                        .chunks(channels)
                        .map(|f| f.iter().sum::<f32>() / channels as f32);
                    push(&mut it)
                },
                err,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let push = push;
            dev.build_input_stream(
                &cfg.into(),
                move |data: &[i16], _| {
                    let mut it = data.chunks(channels).map(|f| {
                        f.iter().map(|s| *s as f32 / 32768.0).sum::<f32>() / channels as f32
                    });
                    push(&mut it)
                },
                err,
                None,
            )
        }
        other => return Err(format!("Format de micro non pris en charge : {other:?}")),
    }
    .map_err(|e| format!("Micro indisponible : {e}"))?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(Recorder {
        _stream: stream,
        buf,
        rate,
    })
}

/// Names of the microphones macOS offers.
pub fn input_devices() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub fn default_input_name() -> Option<String> {
    cpal::default_host().default_input_device().and_then(|d| d.name().ok())
}

/// FluidVoice warns about Bluetooth microphones (headset mode); we can only guess from the name.
pub fn looks_bluetooth(name: &str) -> bool {
    let n = name.to_lowercase();
    ["airpods", "bluetooth", "buds", "headset", "beats", "bose", "sony wh", "jabra"]
        .iter()
        .any(|k| n.contains(k))
}

/// Windowed-sinc (Blackman) resampling to 16 kHz, with a low-pass below the new Nyquist.
pub fn resample(input: &[f32], from: u32) -> Vec<f32> {
    if from == SAMPLE_RATE || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / SAMPLE_RATE as f64;
    let scale = (1.0 / ratio).min(1.0);
    let cutoff = 0.5 * scale * 0.92; // cycles per input sample
    let half = (16.0 / scale).ceil() as isize;
    let n_out = (input.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(n_out);
    for i in 0..n_out {
        let t = i as f64 * ratio;
        let c = t.floor() as isize;
        let (mut acc, mut wsum) = (0.0f64, 0.0f64);
        for j in (c - half)..=(c + half) {
            if j < 0 || j as usize >= input.len() {
                continue;
            }
            let d = t - j as f64;
            let x = 2.0 * cutoff * d;
            let sinc = if x.abs() < 1e-9 { 1.0 } else { (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x) };
            let w = d / (half as f64 + 1.0);
            if w.abs() >= 1.0 {
                continue;
            }
            let win = 0.42 + 0.5 * (std::f64::consts::PI * w).cos() + 0.08 * (2.0 * std::f64::consts::PI * w).cos();
            let h = 2.0 * cutoff * sinc * win;
            acc += input[j as usize] as f64 * h;
            wsum += h;
        }
        out.push(if wsum.abs() > 1e-12 { (acc / wsum) as f32 } else { 0.0 });
    }
    out
}

pub struct Loudness {
    pub rms: f32,
    pub peak: f32,
    pub max_frame_rms: f32,
    pub ms: u64,
    pub finite: bool,
}

/// Same measures as FluidVoice's assessShortAudioSilence (0x10054e294), on 16 kHz audio.
pub fn loudness(pcm: &[f32]) -> Loudness {
    let frame = (SAMPLE_RATE as usize / 50).max(1);
    let (mut sum, mut peak, mut max_frame) = (0.0f64, 0.0f32, 0.0f32);
    let mut finite = true;
    for chunk in pcm.chunks(frame) {
        let mut fs = 0.0f64;
        for &x in chunk {
            if !x.is_finite() {
                finite = false;
            }
            fs += (x as f64) * (x as f64);
            peak = peak.max(x.abs());
        }
        sum += fs;
        max_frame = max_frame.max((fs / chunk.len() as f64).sqrt() as f32);
    }
    Loudness {
        rms: (sum / pcm.len().max(1) as f64).sqrt() as f32,
        peak,
        max_frame_rms: max_frame,
        ms: (pcm.len() as f64 / SAMPLE_RATE as f64 * 1000.0).round() as u64,
        finite,
    }
}

/// Keep the last recording (16 kHz, 16-bit mono WAV) so it can be listened to.
fn save_wav(pcm: &[f32]) -> Option<std::path::PathBuf> {
    let path = whisper::app_dir().join("derniere-dictee.wav");
    let _ = std::fs::create_dir_all(whisper::app_dir());
    let data_len = (pcm.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    b.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for &x in pcm {
        b.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(&path, b).ok().map(|_| path)
}

fn controller(rx: Receiver<Cmd>, jobs: Sender<Job>, shared: SharedRef, ctx: egui::Context) {
    let mut rec: Option<Recorder> = None;
    let mut press_at: Option<Instant> = None;

    let begin = |rec: &mut Option<Recorder>| {
        {
            let sh = shared.lock().unwrap();
            if sh.phase != Phase::Idle {
                return;
            }
            if !matches!(sh.model, ModelState::Ready(_) | ModelState::Loading) {
                drop(sh);
                let mut sh = shared.lock().unwrap();
                sh.message = Some("Téléchargez d'abord un modèle de transcription.".into());
                ctx.request_repaint();
                return;
            }
        }
        match open_mic(shared.clone(), ctx.clone()) {
            Ok(r) => {
                *rec = Some(r);
                let mut sh = shared.lock().unwrap();
                sh.phase = Phase::Recording;
                sh.message = None;
                sh.record_started = Some(Instant::now());
                sh.levels.iter_mut().for_each(|l| *l = 0.0);
                if sh.settings.sounds {
                    tone(true);
                }
            }
            Err(e) => shared.lock().unwrap().message = Some(e),
        }
        ctx.request_repaint();
    };

    let finish = |rec: &mut Option<Recorder>, keep: bool| {
        let Some(r) = rec.take() else { return };
        let samples = std::mem::take(&mut *r.buf.lock().unwrap());
        let rate = r.rate;
        drop(r);
        let mut sh = shared.lock().unwrap();
        let secs = sh
            .record_started
            .take()
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        if sh.settings.sounds {
            tone(false);
        }
        if !keep {
            sh.phase = Phase::Idle;
            sh.message = Some("Enregistrement annulé.".into());
        } else {
            let pcm = resample(&samples, rate);
            sh.last_wav = save_wav(&pcm);
            let l = loudness(&pcm);
            if pcm.len() < MIN_SAMPLES {
                sh.phase = Phase::Idle;
                sh.message = Some("Audio trop court (minimum 1 seconde).".into());
            } else if l.finite && l.rms <= DEAD_MIC_MAX_RMS && l.peak < DEAD_MIC_MAX_PEAK {
                // Exact silence: macOS is not giving us the microphone.
                sh.phase = Phase::Idle;
                sh.mic_dead = true;
                sh.message = Some(
                    "Le micro n'a envoyé aucun son : macOS ne donne pas l'accès au micro à Voix Fluide."
                        .into(),
                );
            } else if sh.settings.skip_silent
                && l.finite
                && l.ms <= SILENCE_MAX_MS
                && l.rms < SILENCE_MAX_RMS
                && l.peak < SILENCE_MAX_PEAK
                && l.max_frame_rms < SILENCE_MAX_FRAME_RMS
            {
                sh.phase = Phase::Idle;
                sh.mic_dead = false;
                sh.message = Some("Silence : rien à transcrire.".into());
            } else {
                sh.mic_dead = false;
                sh.phase = Phase::Transcribing;
                let _ = jobs.send(Job::Transcribe(pcm, secs));
            }
        }
        drop(sh);
        ctx.request_repaint();
    };

    while let Ok(cmd) = rx.recv() {
        let (mode, phase) = {
            let sh = shared.lock().unwrap();
            (sh.settings.mode, sh.phase)
        };
        let recording = phase == Phase::Recording;
        match cmd {
            Cmd::Toggle => {
                press_at = None;
                if recording {
                    finish(&mut rec, true)
                } else {
                    begin(&mut rec)
                }
            }
            Cmd::Cancel => {
                press_at = None;
                finish(&mut rec, false)
            }
            Cmd::KeyDown => match mode {
                Mode::Toggle => {
                    if recording {
                        finish(&mut rec, true)
                    } else {
                        begin(&mut rec)
                    }
                }
                Mode::Hold => {
                    if !recording {
                        begin(&mut rec)
                    }
                }
                Mode::Automatic => {
                    if recording {
                        press_at = None;
                        finish(&mut rec, true)
                    } else {
                        begin(&mut rec);
                        press_at = Some(Instant::now());
                    }
                }
            },
            Cmd::KeyUp => match mode {
                Mode::Hold => {
                    if recording {
                        finish(&mut rec, true)
                    }
                }
                Mode::Automatic => {
                    if let Some(t) = press_at.take() {
                        if recording && t.elapsed().as_secs_f64() >= TAP_HOLD_THRESHOLD {
                            finish(&mut rec, true)
                        }
                    }
                }
                Mode::Toggle => {}
            },
        }
    }
}

fn worker(rx: Receiver<Job>, shared: SharedRef, ctx: egui::Context) {
    let mut engine: Option<Engine> = None;
    let set_model = |s: ModelState| {
        shared.lock().unwrap().model = s;
        ctx.request_repaint();
    };
    let load = |size: ModelSize, engine: &mut Option<Engine>| {
        set_model(ModelState::Loading);
        *engine = None;
        match Engine::load(size, true) {
            Ok(e) => {
                let dev = if e.device.is_metal() { "GPU" } else { "CPU" };
                set_model(ModelState::Ready(format!("{} · {dev}", size.short())));
                *engine = Some(e);
            }
            Err(e) => set_model(ModelState::Failed(e)),
        }
    };
    while let Ok(job) = rx.recv() {
        match job {
            Job::Load(size) => load(size, &mut engine),
            Job::Download(size) => {
                set_model(ModelState::Downloading(0.0));
                let ctx2 = ctx.clone();
                let sh2 = shared.clone();
                let mut last = Instant::now();
                let res = whisper::download(size, |done, total| {
                    if last.elapsed() > Duration::from_millis(100) && total > 0 {
                        last = Instant::now();
                        sh2.lock().unwrap().model =
                            ModelState::Downloading(done as f32 / total as f32);
                        ctx2.request_repaint();
                    }
                });
                match res {
                    Ok(()) => load(size, &mut engine),
                    Err(e) => set_model(ModelState::Failed(e)),
                }
            }
            Job::Transcribe(pcm, secs) => {
                let (lang, paste, keep, key) = {
                    let sh = shared.lock().unwrap();
                    let s = &sh.settings;
                    (s.language.clone(), s.paste, s.keep_clipboard, s.key)
                };
                // The recording may have started while the model was still loading.
                let lang = (lang != "auto").then_some(lang);
                let result = match engine.as_mut() {
                    None => Err("Modèle non chargé.".to_string()),
                    Some(e) => {
                        let first = e.transcribe(&pcm, lang.as_deref());
                        if first.is_err() && e.device.is_metal() {
                            // Fall back to the processor once if the GPU path fails.
                            match Engine::load(e.size, false) {
                                Ok(mut cpu) => {
                                    let r = cpu.transcribe(&pcm, lang.as_deref());
                                    let size = cpu.size;
                                    engine = Some(cpu);
                                    set_model(ModelState::Ready(format!(
                                        "{} · CPU",
                                        size.short()
                                    )));
                                    r
                                }
                                Err(_) => first,
                            }
                        } else {
                            first
                        }
                    }
                };
                let mut sh = shared.lock().unwrap();
                sh.phase = Phase::Idle;
                match result {
                    Ok(text) if text.is_empty() => {
                        sh.message = Some("Aucune parole détectée.".into());
                    }
                    Ok(text) => {
                        sh.last = text.clone();
                        sh.history.insert(
                            0,
                            Entry {
                                text: text.clone(),
                                when: clock(),
                                seconds: secs,
                            },
                        );
                        sh.history.truncate(50);
                        let focused = sh.app_focused;
                        drop(sh);
                        if paste && !focused && mac::accessibility_trusted() {
                            if let Err(e) = mac::paste_text(&text, keep, key) {
                                shared.lock().unwrap().message =
                                    Some(format!("Collage impossible : {e}"));
                            }
                        } else if keep || (paste && !focused) {
                            mac::copy_text(&text);
                            if paste && !focused {
                                shared.lock().unwrap().message = Some(
                                    "Texte copié (⌘V pour coller) : le collage automatique attend l'autorisation Accessibilité, voir la fenêtre Voix Fluide."
                                        .into(),
                                );
                            }
                        }
                    }
                    Err(e) => sh.message = Some(format!("Transcription impossible : {e}")),
                }
                ctx.request_repaint();
            }
        }
    }
}

fn clock() -> String {
    let out = std::process::Command::new("date")
        .arg("+%H:%M")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    out.trim().to_string()
}

/// Short rising (start) or falling (stop) chime.
fn tone(start: bool) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let Some(dev) = host.default_output_device() else { return };
        let Ok(cfg) = dev.default_output_config() else { return };
        if cfg.sample_format() != cpal::SampleFormat::F32 {
            return;
        }
        let rate = cfg.sample_rate().0 as f32;
        let ch = cfg.channels() as usize;
        let total = (rate * 0.12) as usize;
        let mut i = 0usize;
        let (f0, f1) = if start { (660.0, 990.0) } else { (880.0, 587.0) };
        let mut phase = 0.0f32;
        let stream = dev.build_output_stream(
            &cfg.into(),
            move |data: &mut [f32], _| {
                for frame in data.chunks_mut(ch) {
                    let v = if i < total {
                        let t = i as f32 / total as f32;
                        let f = f0 + (f1 - f0) * t;
                        phase += f / rate;
                        let env = (t * 20.0).min(1.0) * (1.0 - t).powf(2.0);
                        (phase * std::f32::consts::TAU).sin() * 0.18 * env
                    } else {
                        0.0
                    };
                    i += 1;
                    frame.iter_mut().for_each(|s| *s = v);
                }
            },
            |e| eprintln!("son: {e}"),
            None,
        );
        if let Ok(s) = stream {
            let _ = s.play();
            std::thread::sleep(Duration::from_millis(220));
        }
    });
}
