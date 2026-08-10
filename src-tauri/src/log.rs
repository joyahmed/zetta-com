//! Everything the app prints, in a file.
//!
//! `main.rs` builds this with `windows_subsystem = "windows"` so that no console
//! window appears behind the app. That flag also means the process starts with
//! **no standard error at all**: every `eprintln!` in this codebase — the lines
//! naming which device was chosen, which stream refused to build, which packet
//! failed to authenticate — has been written to a handle that goes nowhere. The
//! one occasion any of it matters is the one occasion nobody can read it.
//!
//! Pointing stderr at a file is the whole fix, and it is enough on its own:
//! Rust resolves `STD_ERROR_HANDLE` on every write rather than caching it, so
//! this reaches all forty existing lines without editing one of them.

use std::fs::OpenOptions;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE};

/// Small on purpose. This is a diagnostic for the last few runs, not an audit
/// trail, and it lives in the same directory as the settings.
const MAX_BYTES: u64 = 1_000_000;

/// Send stderr to `%APPDATA%\com.joy.zetta-com\zetta-com.log`, and say where.
///
/// Best-effort in every branch: a log that cannot be opened must never be the
/// reason an intercom does not start.
pub fn to_file() -> Option<PathBuf> {
    // Spelled out rather than taken from `app.path().app_config_dir()`, which
    // needs an AppHandle that does not exist yet — and the point is to be
    // logging before anything that can fail has run. It resolves to the same
    // directory: Tauri's config dir on Windows is %APPDATA%\<identifier>.
    let dir = PathBuf::from(std::env::var_os("APPDATA")?).join("com.joy.zetta-com");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("zetta-com.log");

    // Appended rather than truncated: the run that went wrong is usually the
    // one *before* the restart, and truncating at startup throws it away at
    // precisely the moment somebody is restarting to find out what happened.
    // Dropped whole when it grows instead of rotated — half a log with a torn
    // line at the top is not worth the code that produces it.
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        let _ = std::fs::remove_file(&path);
    }

    let file = OpenOptions::new().create(true).append(true).open(&path).ok()?;

    // Leaked deliberately. The handle has to stay open for the whole run, and
    // the process exiting is what closes it; dropping the File here would shut
    // stderr again the moment this returns.
    let handle = HANDLE(file.as_raw_handle());
    std::mem::forget(file);
    unsafe { SetStdHandle(STD_ERROR_HANDLE, handle).ok()? };

    Some(path)
}
