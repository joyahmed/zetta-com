//! Transport settings, on disk.
//!
//! These live on the Rust side rather than in the webview's localStorage
//! because the app has to be able to bind before any window has loaded. A PC
//! whose only job is to listen should come up receiving, without anyone
//! clicking anything — that is how v1's listener behaved, and requiring a
//! button press was a regression dressed up as a feature.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// The port everything binds unless a config says otherwise.
pub const DEFAULT_PORT: u16 = 9001;

/// Deliberately **not** `#[derive(Default)]`.
///
/// `#[serde(default = "…")]` only fills a field that is *missing from a file
/// being parsed*. A derived `Default` ignores those attributes entirely and
/// hands back `u16::default()` and `String::default()` — so
/// `config::load(app).unwrap_or_default()`, which is the path taken on a fresh
/// install where no file exists yet, produced `port: 0` and an **empty
/// talk_shortcut**. An empty spec makes `keys::bind` return early, so the first
/// run of a push-to-talk application had no push-to-talk key, and nothing
/// anywhere reported it: the shortcut list only lists keys that were attempted.
impl Default for Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            peer: String::new(),
            manual: Vec::new(),
            labels: HashMap::new(),
            talk_shortcut: default_talk_shortcut(),
            presets: default_presets(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub port: u16,
    pub peer: String,
    /// Addresses typed in manually, kept alongside whatever mDNS finds.
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

    /// Your own name for a machine, by address — "Rafi" rather than "DEVS002".
    ///
    /// A PC name is what the machine calls itself, which is rarely what you
    /// call the person sitting at it. This wins over both the discovered name
    /// and the one carried in heartbeats, because it is the only one somebody
    /// chose on purpose.
    #[serde(default)]
    pub labels: HashMap<String, String>,

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

/// Empty on purpose.
///
/// This is a tool for directing people, not for chatting with them, so shipped
/// pleasantries were wrong. The mechanism stays — anyone who wants a key that
/// fires "come to my desk" can add one — but nothing is invented on their
/// behalf.
fn default_presets() -> Vec<Preset> {
    Vec::new()
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

/// A port given on the command line, which wins over the saved one and is not
/// written back.
///
/// Two instances on one machine share this config file, so without an override
/// the second one either fails to bind or saves its port over the first's. That
/// only happens while testing, and it is exactly when it is most confusing.
///
/// `zetta-com.exe --port 9002`
pub fn port_override() -> Option<u16> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--port" {
            return args.next().and_then(|v| v.parse().ok());
        }
        if let Some(v) = a.strip_prefix("--port=") {
            return v.parse().ok();
        }
    }
    None
}

/// `None` when there is nothing saved yet, or when what is saved cannot be
/// read. A corrupt config must not stop the app launching — it just means the
/// window opens stopped, which is recoverable manually.
pub fn load(app: &AppHandle) -> Option<Config> {
    let p = path(app).ok()?;
    let raw = fs::read_to_string(&p).ok()?;
    match serde_json::from_str::<Config>(&raw) {
        Ok(mut c) => {
            // Drop the pleasantries an earlier build shipped. Removing them
            // from the defaults did nothing for anyone who had already run it,
            // because they were written to disk on the first successful bind.
            const SHIPPED: [&str; 3] = ["On my way", "Lunch?", "Call me"];
            c.presets.retain(|p| {
                !SHIPPED.contains(&p.text.as_str()) && !SHIPPED.contains(&p.label.as_str())
            });

            // Repair a config written before `Config` had a real `Default`.
            // The derived one produced an empty talk shortcut, and any save
            // after that wrote the empty string to disk — where serde reads it
            // back happily, because the field is present. Fixing the default
            // alone would leave every machine that has already run the app
            // without a push-to-talk key, permanently and silently.
            if c.talk_shortcut.trim().is_empty() {
                eprintln!("[config] talk shortcut was empty, restoring the default");
                c.talk_shortcut = default_talk_shortcut();
            }
            if c.port == 0 {
                c.port = DEFAULT_PORT;
            }

            Some(c)
        }
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
