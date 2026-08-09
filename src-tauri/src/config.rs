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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub port: u16,
    pub peer: String,
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
