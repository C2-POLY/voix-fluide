//! The Voix Fluide icon in the menu bar and its small menu: the state, copy the last
//! transcription, open the window, pick the microphone, quit.
//!
//! The icon and the menu live on the main thread (created from `App::new`). A click
//! reaches muda's event handler, which forwards the item id through a channel and asks
//! egui for a repaint, so `App::update` runs (even when the window is minimized) and
//! reads the clicks with `Tray::poll`.

use crate::engine::{ModelState, Phase};
use crate::icon;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use tray_icon::menu::accelerator::{Accelerator, Code, Modifiers};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Height of the menu bar picture in pixels: tray-icon draws it 18 pt tall, so this is 2x.
const ICON_PX: u32 = 36;

const DEFAULT_MIC: &str = "Par défaut du système";

/// What a click in the menu asks the app to do.
#[derive(Clone, Debug, PartialEq)]
pub enum TrayAction {
    CopyLast,
    ShowWindow,
    /// None = the system default.
    SetMic(Option<String>),
    Quit,
}

/// The disabled first line of the menu.
pub fn status_text(phase: Phase, model: &ModelState, key_label: &str) -> String {
    match phase {
        Phase::Recording => return "Écoute…".into(),
        Phase::Transcribing => return "Transcription…".into(),
        Phase::Idle => {}
    }
    match model {
        ModelState::Ready(_) => format!("Prête ({key_label})"),
        ModelState::Missing => "Modèle manquant".into(),
        ModelState::Downloading(p) => format!("Téléchargement du modèle… {:.0} %", p * 100.0),
        ModelState::Loading => "Chargement du modèle…".into(),
        ModelState::Failed(_) => "Modèle en échec".into(),
    }
}

/// One line of the Micro submenu.
#[derive(Clone, Debug, PartialEq)]
pub struct MicEntry {
    pub label: String,
    /// The value `settings.mic` takes when this line is picked.
    pub mic: Option<String>,
    pub checked: bool,
}

/// The Micro submenu: the system default, then each detected device. A chosen
/// device that is not plugged in stays listed (checked) so the menu tells the setting.
pub fn mic_entries(devices: &[String], chosen: Option<&str>) -> Vec<MicEntry> {
    let mut out = vec![MicEntry { label: DEFAULT_MIC.into(), mic: None, checked: chosen.is_none() }];
    for d in devices {
        out.push(MicEntry { label: d.clone(), mic: Some(d.clone()), checked: chosen == Some(d.as_str()) });
    }
    if let Some(c) = chosen {
        if !devices.iter().any(|d| d == c) {
            out.push(MicEntry { label: c.to_string(), mic: Some(c.to_string()), checked: true });
        }
    }
    out
}

pub struct Tray {
    _icon: TrayIcon,
    events: Receiver<MenuId>,
    status: MenuItem,
    copy: MenuItem,
    show: MenuItem,
    mic_menu: Submenu,
    mic_items: Vec<CheckMenuItem>,
    /// Micro submenu item id → the `settings.mic` value it picks.
    mic_ids: HashMap<MenuId, Option<String>>,
    quit: MenuItem,
    // Last values put in the menu, to touch it only when something changed.
    last_status: String,
    last_copy: bool,
    last_mics: Vec<MicEntry>,
}

impl Tray {
    /// Creates the icon and its menu. Must run on the main thread.
    pub fn new(ctx: &egui::Context) -> Result<Self, String> {
        let status = MenuItem::new("", false, None);
        let copy = MenuItem::new("Copier la dernière transcription", false, None);
        let show = MenuItem::new("Ouvrir Voix Fluide", true, None);
        let mic_menu = Submenu::new("Micro", true);
        let quit = MenuItem::new("Quitter Voix Fluide", true, Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyQ)));
        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &copy,
            &PredefinedMenuItem::separator(),
            &show,
            &mic_menu,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| e.to_string())?;

        let (rgba, w, h) = icon::render_template(ICON_PX);
        let icon = Icon::from_rgba(rgba, w, h).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_icon_as_template(true)
            .with_tooltip("Voix Fluide")
            .build()
            .map_err(|e| e.to_string())?;

        let (tx, rx) = channel();
        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let _ = tx.send(e.id);
            ctx.request_repaint();
        }));
        // Clicks on the icon itself only open the menu: drop them instead of queuing them.
        TrayIconEvent::set_event_handler(Some(|_: TrayIconEvent| {}));

        Ok(Self {
            _icon: tray,
            events: rx,
            status,
            copy,
            show,
            mic_menu,
            mic_items: Vec::new(),
            mic_ids: HashMap::new(),
            quit,
            last_status: String::new(),
            last_copy: false,
            last_mics: Vec::new(),
        })
    }

    /// Puts the current state in the menu, touching only what changed.
    pub fn refresh(&mut self, phase: Phase, model: &ModelState, key_label: &str, has_last: bool, devices: &[String], mic: Option<&str>) {
        let status = status_text(phase, model, key_label);
        if status != self.last_status {
            self.status.set_text(&status);
            self.last_status = status;
        }
        if has_last != self.last_copy {
            self.copy.set_enabled(has_last);
            self.last_copy = has_last;
        }
        let mics = mic_entries(devices, mic);
        if mics != self.last_mics {
            let same_labels = mics.len() == self.last_mics.len() && mics.iter().zip(&self.last_mics).all(|(a, b)| a.label == b.label);
            if same_labels {
                for (item, e) in self.mic_items.iter().zip(&mics) {
                    item.set_checked(e.checked);
                }
            } else {
                for item in self.mic_items.drain(..) {
                    let _ = self.mic_menu.remove(&item);
                }
                self.mic_ids.clear();
                for e in &mics {
                    let item = CheckMenuItem::new(&e.label, true, e.checked, None);
                    let _ = self.mic_menu.append(&item);
                    self.mic_ids.insert(item.id().clone(), e.mic.clone());
                    self.mic_items.push(item);
                }
            }
            self.last_mics = mics;
        }
    }

    /// The clicks since the last call.
    pub fn poll(&mut self) -> Vec<TrayAction> {
        let mut out = Vec::new();
        while let Ok(id) = self.events.try_recv() {
            if id == *self.copy.id() {
                out.push(TrayAction::CopyLast);
            } else if id == *self.show.id() {
                out.push(TrayAction::ShowWindow);
            } else if id == *self.quit.id() {
                out.push(TrayAction::Quit);
            } else if let Some(mic) = self.mic_ids.get(&id) {
                out.push(TrayAction::SetMic(mic.clone()));
                // macOS flips the clicked check mark by itself: put the marks back
                // from the settings at the next refresh, even if the choice is the same.
                for (e, item) in self.last_mics.iter_mut().zip(&self.mic_items) {
                    e.checked = item.is_checked();
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_follows_phase_then_model() {
        let ready = ModelState::Ready("Turbo · GPU".into());
        assert_eq!(status_text(Phase::Idle, &ready, "⌥ Option droite"), "Prête (⌥ Option droite)");
        assert_eq!(status_text(Phase::Recording, &ready, "x"), "Écoute…");
        assert_eq!(status_text(Phase::Transcribing, &ready, "x"), "Transcription…");
        // Recording and transcribing win over the model state.
        assert_eq!(status_text(Phase::Recording, &ModelState::Loading, "x"), "Écoute…");
        assert_eq!(status_text(Phase::Idle, &ModelState::Missing, "x"), "Modèle manquant");
        assert_eq!(status_text(Phase::Idle, &ModelState::Downloading(0.42), "x"), "Téléchargement du modèle… 42 %");
        assert_eq!(status_text(Phase::Idle, &ModelState::Loading, "x"), "Chargement du modèle…");
        assert_eq!(status_text(Phase::Idle, &ModelState::Failed("boom".into()), "x"), "Modèle en échec");
    }

    fn labels(e: &[MicEntry]) -> Vec<(&str, bool)> {
        e.iter().map(|e| (e.label.as_str(), e.checked)).collect()
    }

    #[test]
    fn mic_default_checked() {
        let devs = vec!["MacBook Pro Microphone".to_string(), "USB Mic".to_string()];
        let e = mic_entries(&devs, None);
        assert_eq!(labels(&e), vec![(DEFAULT_MIC, true), ("MacBook Pro Microphone", false), ("USB Mic", false)]);
        assert_eq!(e[0].mic, None);
        assert_eq!(e[2].mic.as_deref(), Some("USB Mic"));
    }

    #[test]
    fn mic_chosen_device_checked() {
        let devs = vec!["MacBook Pro Microphone".to_string(), "USB Mic".to_string()];
        let e = mic_entries(&devs, Some("USB Mic"));
        assert_eq!(labels(&e), vec![(DEFAULT_MIC, false), ("MacBook Pro Microphone", false), ("USB Mic", true)]);
    }

    #[test]
    fn mic_unplugged_choice_stays_listed() {
        let devs = vec!["MacBook Pro Microphone".to_string()];
        let e = mic_entries(&devs, Some("AirPods"));
        assert_eq!(labels(&e), vec![(DEFAULT_MIC, false), ("MacBook Pro Microphone", false), ("AirPods", true)]);
        assert_eq!(e[2].mic.as_deref(), Some("AirPods"));
    }

    #[test]
    fn mic_no_device() {
        assert_eq!(labels(&mic_entries(&[], None)), vec![(DEFAULT_MIC, true)]);
    }
}
