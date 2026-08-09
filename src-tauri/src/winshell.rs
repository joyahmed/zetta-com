//! Making Windows toasts say "Zetta Com" instead of "Windows PowerShell".
//!
//! Windows attributes a toast to an **AppUserModelID**, and it will only show a
//! name and icon for an AUMID it can resolve to a Start Menu shortcut carrying
//! that same ID. A desktop app that never sets one has no AUMID at all, so the
//! notification library falls back to PowerShell's — which is why the toast was
//! captioned "Windows PowerShell". Windows was not guessing; it was told.
//!
//! Both halves are required and neither works alone:
//!
//!   1. a Start Menu shortcut whose `System.AppUserModel.ID` is our identifier,
//!   2. the process declaring that same identifier at startup.
//!
//! With only the shortcut, the process still has no ID. With only the process
//! call, the ID resolves to nothing and Windows has no name to show.
//!
//! The shortcut is written to the *user's* Start Menu rather than the machine's.
//! The installer already puts one in ProgramData, but writing there needs
//! administrator rights the running app does not have, and a per-user shortcut
//! is enough for Windows to resolve the ID.

use std::path::PathBuf;

use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::System::Com::StructuredStorage::{InitPropVariantFromStringVector, PropVariantClear};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, IPersistFile,
};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    IShellLinkW, SetCurrentProcessExplicitAppUserModelID, ShellLink,
};

/// `System.AppUserModel.ID`. Not exposed as a constant by the `windows` crate,
/// so it is spelled out — it is a fixed, documented value.
const PKEY_APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
    fmtid: windows::core::GUID::from_u128(0x9F4C2855_9F79_4B39_A8D0_E1D42DE1D5F3),
    pid: 5,
};

/// Everything below is best-effort. A missing shortcut or a refused COM call
/// costs a nicely-labelled notification; it must never stop the intercom
/// starting, which is the thing anybody actually needs.
pub fn register(app_id: &str, display_name: &str) {
    unsafe {
        // Ignored deliberately: another component may have initialised COM on
        // this thread already, and RPC_E_CHANGED_MODE is not a failure for us.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    if let Err(e) = ensure_shortcut(app_id, display_name) {
        eprintln!("[win] could not write the Start Menu shortcut: {e}");
    }

    let id = HSTRING::from(app_id);
    if let Err(e) = unsafe { SetCurrentProcessExplicitAppUserModelID(PCWSTR(id.as_ptr())) } {
        eprintln!("[win] could not set the app id: {e}");
    }
}

fn shortcut_path(display_name: &str) -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(appdata)
            .join("Microsoft/Windows/Start Menu/Programs")
            .join(format!("{display_name}.lnk")),
    )
}

/// Written every launch rather than only when missing. It is a few milliseconds,
/// and it repairs the case that would otherwise be permanent and invisible: a
/// shortcut left by an older build with no AUMID on it, which resolves to
/// nothing and sends you straight back to the PowerShell caption.
fn ensure_shortcut(app_id: &str, display_name: &str) -> windows::core::Result<()> {
    let Some(path) = shortcut_path(display_name) else {
        return Ok(());
    };
    let exe = std::env::current_exe().unwrap_or_default();

    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;

        let exe_h = HSTRING::from(exe.as_os_str());
        link.SetPath(PCWSTR(exe_h.as_ptr()))?;
        if let Some(dir) = exe.parent() {
            let dir_h = HSTRING::from(dir.as_os_str());
            link.SetWorkingDirectory(PCWSTR(dir_h.as_ptr()))?;
        }

        // The AUMID is a property on the link, not part of the link itself.
        // This is the piece the installer's shortcut is missing.
        let store: IPropertyStore = link.cast()?;
        let id = HSTRING::from(app_id);
        let mut value = InitPropVariantFromStringVector(Some(&[PCWSTR(id.as_ptr())]))?;
        store.SetValue(&PKEY_APP_USER_MODEL_ID, &value)?;
        store.Commit()?;
        let _ = PropVariantClear(&mut value);

        let file: IPersistFile = link.cast()?;
        let path_h = HSTRING::from(path.as_os_str());
        file.Save(PCWSTR(path_h.as_ptr()), true)?;
    }
    Ok(())
}
