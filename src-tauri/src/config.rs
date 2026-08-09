//! Transport settings, on disk.
//!
//! These live on the Rust side rather than in the webview's localStorage
//! because the app has to be able to bind before any window has loaded. A PC
//! whose only job is to listen should come up receiving, without anyone
//! clicking anything — that is how v1's listener behaved, and requiring a
//! button press was a regression dressed up as a feature.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub port: u16,
    pub peer: String,
    /// Addresses typed in by hand, kept alongside whatever mDNS finds.
    ///
    /// Discovery is not enough on its own: plenty of networks filter mDNS, a PC
    /// on another subnet is never discovered, and v1 kept a hand-maintained
    /// roster for exactly those reasons. A typed address is also the only fix
    /// for the v1 case of a machine whose name would not resolve at all.
    ///
    /// `default` so a config written before this field existed still loads —
    /// otherwise adding a setting would silently reset everyone's port.
    #[serde(default)]
    pub manual: Vec<String>,

    /// Hold this to talk. A single key by default so it can be *held*, and one
    /// nothing else tends to claim.
    #[serde(default = "default_talk_shortcut")]
    pub talk_shortcut: String,

    /// Canned messages, each on its own key. This is what "auto message" means
    /// here: a short editable list sent with one keystroke and no typing —
    /// not auto-replies, and not scheduled announcements.
    #[serde(default = "default_presets")]
    pub presets: Vec<Preset>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub label: String,
    pub text: String,
    /// Empty means the preset exists but has no key — useful, since there are
    /// only so many combinations worth spending.
    #[serde(default)]
    pub shortcut: String,
}

fn default_talk_shortcut() -> String {
    "F8".to_string()
}

fn default_presets() -> Vec<Preset> {
    [
        ("On my way", "On my way", "CommandOrControl+Alt+1"),
        ("Lunch?", "Lunch?", "CommandOrControl+Alt+2"),
        ("Call me", "Call me", "CommandOrControl+Alt+3"),
    ]
    .into_iter()
    .map(|(label, text, shortcut)| Preset {
        label: label.to_string(),
        text: text.to_string(),
        shortcut: shortcut.to_string(),
    })
    .collect()
}

fn path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("no app config directory")?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir.join("transport.json"))
}

/// `None` when there is nothing saved yet, or when what is saved cannot be
/// read. A corrupt config must not stop the app launching — it just means the
/// window opens stopped, which is recoverable by hand.
pub fn load(app: &AppHandle) -> Option<Config> {
    let p = path(app).ok()?;
    let raw = fs::read_to_string(&p).ok()?;
    match serde_json::from_str::<Config>(&raw) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("[config] ignoring unreadable {}: {e}", p.display());
            None
        }
    }
}

pub fn save(app: &AppHandle, cfg: &Config) -> Result<()> {
    let p = path(app)?;
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(&p, json).with_context(|| format!("writing {}", p.display()))?;
    Ok(())
}
