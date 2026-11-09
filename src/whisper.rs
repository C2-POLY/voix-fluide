//! Local transcription models, with download: Whisper (candle, pure Rust)
//! and NVIDIA Parakeet TDT v3 (ONNX, the model FluidVoice uses by default).

use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as w, audio, model::Whisper};
use parakeet_rs::{ParakeetTDT, Transcriber};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModelSize {
    Tiny,
    Base,
    Small,
    Parakeet,
}

impl ModelSize {
    pub const ALL: [ModelSize; 4] = [
        ModelSize::Parakeet,
        ModelSize::Tiny,
        ModelSize::Base,
        ModelSize::Small,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ModelSize::Tiny => "tiny",
            ModelSize::Base => "base",
            ModelSize::Small => "small",
            ModelSize::Parakeet => "parakeet-tdt-v3",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.id() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            ModelSize::Tiny => "Whisper Tiny (~150 Mo) — le plus rapide",
            ModelSize::Base => "Whisper Base (~290 Mo)",
            ModelSize::Small => "Whisper Small (~970 Mo)",
            ModelSize::Parakeet => "NVIDIA Parakeet TDT v3 (~670 Mo) — 25 langues",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            ModelSize::Tiny => "Whisper Tiny",
            ModelSize::Base => "Whisper Base",
            ModelSize::Small => "Whisper Small",
            ModelSize::Parakeet => "Parakeet TDT v3",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            ModelSize::Parakeet => "Modèle NVIDIA (version compacte int8), langue détectée automatiquement. Licence CC-BY-4.0.",
            _ => "Whisper d'OpenAI, 99 langues.",
        }
    }

    pub fn is_parakeet(self) -> bool {
        self == ModelSize::Parakeet
    }

    fn files(self) -> &'static [&'static str] {
        match self {
            ModelSize::Parakeet => &[
                "vocab.txt",
                "decoder_joint-model.int8.onnx",
                "encoder-model.int8.onnx",
            ],
            _ => &["config.json", "tokenizer.json", "model.safetensors"],
        }
    }

    /// The big file whose progress is shown.
    fn big_file(self) -> &'static str {
        match self {
            ModelSize::Parakeet => "encoder-model.int8.onnx",
            _ => "model.safetensors",
        }
    }

    fn url(self, file: &str) -> String {
        match self {
            ModelSize::Parakeet => format!(
                "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/{file}"
            ),
            _ => format!(
                "https://huggingface.co/openai/whisper-{}/resolve/main/{file}",
                self.id()
            ),
        }
    }
}

pub fn app_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Library/Application Support/VoixFluide")
}

pub fn model_dir(size: ModelSize) -> PathBuf {
    let name = if size.is_parakeet() {
        size.id().to_string()
    } else {
        format!("whisper-{}", size.id())
    };
    app_dir().join("models").join(name)
}

pub fn is_downloaded(size: ModelSize) -> bool {
    let d = model_dir(size);
    size.files().iter().all(|f| d.join(f).exists())
}

/// Download the model files; `progress(done, total)` is called as the big file arrives.
pub fn download(size: ModelSize, mut progress: impl FnMut(u64, u64)) -> Result<(), String> {
    let dir = model_dir(size);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for &f in size.files() {
        let dest = dir.join(f);
        if dest.exists() {
            continue;
        }
        let url = size.url(f);
        let resp = ureq::get(&url)
            .call()
            .map_err(|e| format!("Téléchargement impossible ({f}) : {e}"))?;
        let total: u64 = resp
            .header("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let part = dir.join(format!("{f}.part"));
        let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        let mut reader = resp.into_reader();
        let mut buf = vec![0u8; 1 << 16];
        let mut done = 0u64;
        loop {
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            if f == size.big_file() {
                progress(done, total);
            }
        }
        out.flush().map_err(|e| e.to_string())?;
        std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Slaney-style mel filter bank (same shape Whisper was trained with).
fn mel_filters(n_mels: usize, n_fft: usize, sr: f32) -> Vec<f32> {
    let n_bins = n_fft / 2 + 1;
    let hz_to_mel = |f: f32| -> f32 {
        let f_sp = 200.0 / 3.0;
        let min_log_hz = 1000.0;
        let min_log_mel = min_log_hz / f_sp;
        let logstep = (6.4f32).ln() / 27.0;
        if f >= min_log_hz {
            min_log_mel + (f / min_log_hz).ln() / logstep
        } else {
            f / f_sp
        }
    };
    let mel_to_hz = |m: f32| -> f32 {
        let f_sp = 200.0 / 3.0;
        let min_log_hz = 1000.0;
        let min_log_mel = min_log_hz / f_sp;
        let logstep = (6.4f32).ln() / 27.0;
        if m >= min_log_mel {
            min_log_hz * (logstep * (m - min_log_mel)).exp()
        } else {
            f_sp * m
        }
    };
    let fmax = sr / 2.0;
    let (lo, hi) = (hz_to_mel(0.0), hz_to_mel(fmax));
    let mel_f: Vec<f32> = (0..n_mels + 2)
        .map(|i| mel_to_hz(lo + (hi - lo) * i as f32 / (n_mels + 1) as f32))
        .collect();
    let fft_f: Vec<f32> = (0..n_bins)
        .map(|j| fmax * j as f32 / (n_bins - 1) as f32)
        .collect();
    let mut out = vec![0f32; n_mels * n_bins];
    for i in 0..n_mels {
        let enorm = 2.0 / (mel_f[i + 2] - mel_f[i]);
        for (j, &f) in fft_f.iter().enumerate() {
            let lower = (f - mel_f[i]) / (mel_f[i + 1] - mel_f[i]);
            let upper = (mel_f[i + 2] - f) / (mel_f[i + 2] - mel_f[i + 1]);
            out[i * n_bins + j] = lower.min(upper).max(0.0) * enorm;
        }
    }
    out
}

/// GPT-2 byte-level decoding table: printable char -> original byte.
fn byte_decoder() -> HashMap<char, u8> {
    let mut bs: Vec<u32> = (b'!' as u32..=b'~' as u32)
        .chain(0xA1..=0xAC)
        .chain(0xAE..=0xFF)
        .collect();
    let mut cs = bs.clone();
    let mut n = 0;
    for b in 0..256u32 {
        if !bs.contains(&b) {
            bs.push(b);
            cs.push(256 + n);
            n += 1;
        }
    }
    bs.into_iter()
        .zip(cs)
        .map(|(b, c)| (char::from_u32(c).unwrap(), b as u8))
        .collect()
}

struct Tokens {
    id_to_piece: HashMap<u32, String>,
    special: HashMap<String, u32>,
    bytes: HashMap<char, u8>,
    languages: Vec<(String, u32)>,
}

impl Tokens {
    fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let mut id_to_piece = HashMap::new();
        if let Some(vocab) = json["model"]["vocab"].as_object() {
            for (k, v) in vocab {
                if let Some(id) = v.as_u64() {
                    id_to_piece.insert(id as u32, k.clone());
                }
            }
        }
        let mut special = HashMap::new();
        let mut languages = Vec::new();
        if let Some(added) = json["added_tokens"].as_array() {
            for t in added {
                if let (Some(c), Some(id)) = (t["content"].as_str(), t["id"].as_u64()) {
                    special.insert(c.to_string(), id as u32);
                    let inner = c.trim_start_matches("<|").trim_end_matches("|>");
                    if c.starts_with("<|")
                        && (2..=3).contains(&inner.len())
                        && inner.chars().all(|ch| ch.is_ascii_lowercase())
                    {
                        languages.push((inner.to_string(), id as u32));
                    }
                }
            }
        }
        Ok(Self {
            id_to_piece,
            special,
            bytes: byte_decoder(),
            languages,
        })
    }

    fn special(&self, s: &str) -> Result<u32, String> {
        self.special
            .get(s)
            .copied()
            .ok_or_else(|| format!("jeton {s} absent du modèle"))
    }

    fn decode(&self, ids: &[u32], eot: u32) -> String {
        let mut bytes = Vec::new();
        for id in ids {
            if *id >= eot {
                continue;
            }
            if let Some(p) = self.id_to_piece.get(id) {
                for ch in p.chars() {
                    if let Some(b) = self.bytes.get(&ch) {
                        bytes.push(*b);
                    }
                }
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// A loaded transcription model (Whisper or Parakeet).
pub struct Engine {
    kind: Kind,
    pub device: Device,
    pub size: ModelSize,
}

// Only one engine exists at a time, so the size gap between variants doesn't matter.
#[allow(clippy::large_enum_variant)]
enum Kind {
    Whisper(WhisperEngine),
    Parakeet(Box<ParakeetTDT>),
}

impl Engine {
    pub fn load(size: ModelSize, prefer_gpu: bool) -> Result<Self, String> {
        if size.is_parakeet() {
            let p = ParakeetTDT::from_pretrained(model_dir(size), None)
                .map_err(|e| e.to_string())?;
            return Ok(Self {
                kind: Kind::Parakeet(Box::new(p)),
                device: Device::Cpu,
                size,
            });
        }
        let device = if prefer_gpu {
            Device::new_metal(0).unwrap_or(Device::Cpu)
        } else {
            Device::Cpu
        };
        let we = WhisperEngine::load(model_dir(size), device.clone())?;
        Ok(Self {
            kind: Kind::Whisper(we),
            device,
            size,
        })
    }

    /// `pcm` is 16 kHz mono. `language` is a code like "fr", or None to detect it
    /// (Parakeet always detects the language itself).
    pub fn transcribe(&mut self, pcm: &[f32], language: Option<&str>) -> Result<String, String> {
        match &mut self.kind {
            Kind::Whisper(we) => we.run(pcm, language).map_err(|e| e.to_string()),
            Kind::Parakeet(p) => p
                .transcribe_samples(pcm.to_vec(), 16000, 1, None)
                .map(|r| clean(&r.text))
                .map_err(|e| e.to_string()),
        }
    }
}

struct WhisperEngine {
    model: Whisper,
    config: w::Config,
    tokens: Tokens,
    filters: Vec<f32>,
    device: Device,
}

impl WhisperEngine {
    fn load(dir: PathBuf, device: Device) -> Result<Self, String> {
        let config: w::Config = serde_json::from_str(
            &std::fs::read_to_string(dir.join("config.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let tokens = Tokens::load(&dir.join("tokenizer.json"))?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[dir.join("model.safetensors")], w::DTYPE, &device)
        }
        .map_err(|e| e.to_string())?;
        let model = Whisper::load(&vb, config.clone()).map_err(|e| e.to_string())?;
        let filters = mel_filters(config.num_mel_bins, w::N_FFT, w::SAMPLE_RATE as f32);
        Ok(Self {
            model,
            config,
            tokens,
            filters,
            device,
        })
    }

    fn run(&mut self, pcm: &[f32], language: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
        let n_mels = self.config.num_mel_bins;
        let mel = audio::pcm_to_mel(&self.config, pcm, &self.filters);
        let mel_len = mel.len() / n_mels;
        let mel = Tensor::from_vec(mel, (1, n_mels, mel_len), &self.device)?;
        let content_frames = (pcm.len() / w::HOP_LENGTH).max(1);

        let sot = self.tokens.special(w::SOT_TOKEN)?;
        let transcribe = self.tokens.special(w::TRANSCRIBE_TOKEN)?;
        let no_ts = self.tokens.special(w::NO_TIMESTAMPS_TOKEN)?;
        let eot = self.tokens.special(w::EOT_TOKEN)?;

        let vocab = self.config.vocab_size;
        let mut suppress = vec![0f32; vocab];
        for &t in &self.config.suppress_tokens {
            if (t as usize) < vocab {
                suppress[t as usize] = f32::NEG_INFINITY;
            }
        }
        suppress[no_ts as usize] = f32::NEG_INFINITY;
        let suppress = Tensor::new(suppress.as_slice(), &self.device)?;

        let mut text = String::new();
        let mut seek = 0;
        let mut lang_token: Option<u32> = language.and_then(|l| {
            self.tokens
                .languages
                .iter()
                .find(|(c, _)| c == l)
                .map(|(_, id)| *id)
        });
        while seek < content_frames {
            let seg = (content_frames - seek).min(w::N_FRAMES);
            let seg_frames = w::N_FRAMES.min(mel_len - seek);
            let mel_seg = mel.narrow(2, seek, seg_frames)?;
            let features = self.model.encoder.forward(&mel_seg, true)?;

            if lang_token.is_none() && !self.tokens.languages.is_empty() {
                let t = Tensor::new(&[sot], &self.device)?.unsqueeze(0)?;
                let ys = self.model.decoder.forward(&t, &features, true)?;
                let logits = self.model.decoder.final_linear(&ys.i((..1, 0..1))?)?.i(0)?.i(0)?;
                let ids: Vec<u32> = self.tokens.languages.iter().map(|(_, id)| *id).collect();
                let idx = Tensor::new(ids.as_slice(), &self.device)?;
                let lang_logits: Vec<f32> = logits.index_select(&idx, 0)?.to_vec1()?;
                let best = lang_logits
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(i, _)| ids[i]);
                lang_token = best;
            }

            let mut tokens = vec![sot];
            if let Some(l) = lang_token {
                tokens.push(l);
            }
            tokens.push(transcribe);
            tokens.push(no_ts);
            let prompt_len = tokens.len();
            let max_len = self.config.max_target_positions / 2;
            let no_speech = w::NO_SPEECH_TOKENS
                .iter()
                .find_map(|t| self.tokens.special.get(*t).copied());
            let mut no_speech_prob = 0.0f64;
            let (mut sum_logprob, mut n_logprob) = (0.0f64, 0usize);
            for i in 0..max_len {
                let t = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;
                let ys = self.model.decoder.forward(&t, &features, i == 0)?;
                let (_, seq_len, _) = ys.dims3()?;
                let logits = self
                    .model
                    .decoder
                    .final_linear(&ys.i((..1, seq_len - 1..))?)?
                    .i(0)?
                    .i(0)?;
                if i == 0 {
                    // Whisper's own "no speech" probability, read right after <|startoftranscript|>.
                    if let Some(ns) = no_speech {
                        let at_sot = self
                            .model
                            .decoder
                            .final_linear(&ys.i((..1, 0..1))?)?
                            .i(0)?
                            .i(0)?;
                        let probs = candle_nn::ops::softmax(&at_sot, 0)?;
                        no_speech_prob = probs.i(ns as usize)?.to_scalar::<f32>()? as f64;
                    }
                }
                let logits = logits.broadcast_add(&suppress)?;
                let next = logits.argmax(0)?.to_dtype(DType::U32)?.to_scalar::<u32>()?;
                let logp = candle_nn::ops::log_softmax(&logits, 0)?
                    .i(next as usize)?
                    .to_scalar::<f32>()? as f64;
                sum_logprob += logp;
                n_logprob += 1;
                if next == eot {
                    break;
                }
                tokens.push(next);
                if tokens.len() >= self.config.max_target_positions {
                    break;
                }
            }
            let avg_logprob = sum_logprob / n_logprob.max(1) as f64;
            // Same rule as Whisper: likely silence and an unsure guess -> drop it.
            let silent = (no_speech_prob > w::NO_SPEECH_THRESHOLD && avg_logprob < w::LOGPROB_THRESHOLD)
                || no_speech_prob > NO_SPEECH_ALONE;
            if std::env::var_os("VF_DEBUG").is_some() {
                eprintln!("no_speech={no_speech_prob:.3} avg_logprob={avg_logprob:.3} token={no_speech:?}");
            }
            if !silent {
                let piece = self.tokens.decode(&tokens[prompt_len..], eot);
                text.push_str(&piece);
            }
            seek += seg;
        }
        self.model.reset_kv_cache();
        Ok(clean(&text))
    }
}

/// Estimated: a "no speech" score this high means silence even when Whisper is
/// confident in its guess (it says "you" with confidence on pure silence).
const NO_SPEECH_ALONE: f64 = 0.85;

fn clean(s: &str) -> String {
    let t = s.trim();
    // Whisper sometimes writes these on silence.
    let lowered = t.to_lowercase();
    for junk in ["[blank_audio]", "[silence]", "(silence)", "[music]", "[musique]"] {
        if lowered == junk {
            return String::new();
        }
    }
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}
