//! Wiring only.
//!
//! The parts each own one job: `audio` captures and plays, `net` moves bytes,
//! `session` joins those two, `discovery` finds machines, `commands` is what
//! the window can ask for, `keys` is what a keypress does. This file builds the
//! app out of them and does nothing else — it used to hold all of it, and at
//! seven hundred lines it had stopped being readable.

mod audio;
mod commands;
mod config;
mod discovery;
mod keys;
mod net;
mod room;
mod session;
mod state;
#[cfg(target_os = "windows")]
mod winshell;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::ShortcutState;

use keys::{reveal, Bindings, BindingsState, ShortcutList, Shortcuts};
use state::{NetState, Ptt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Created before the builder because both the shortcut handler and the
    // session need them, and the handler is installed while the app is being
    // configured rather than after.
    // Before anything else: Windows resolves a toast's name and icon from the
    // process AppUserModelID, and without one the notification library falls
    // back to PowerShell's — which is why every toast was captioned "Windows
    // PowerShell".
    #[cfg(target_os = "windows")]
    winshell::register("com.joy.zetta-com", "Zetta Com");

    let ptt = Arc::new(AtomicBool::new(false));
    let bindings: Bindings = Arc::new(Mutex::new(Vec::new()));
    let listing: ShortcutList = Arc::new(Mutex::new(Vec::new()));

    let ptt_for_handler = ptt.clone();
    let bindings_for_handler = bindings.clone();

    let mut builder = tauri::Builder::default();

    // One instance, unless a port was given on the command line.
    //
    // Closing the window hides to the tray, so a running app is invisible — and
    // every later launch would take a second copy that cannot bind the port or
    // claim the hotkeys, and would look simply broken. v1 hit this too: two
    // copies of the listener were the cause of its reported echo, and a mutex
    // was the fix there as this is here.
    //
    // The override is the exception on purpose. Running two deliberately, for
    // testing, is the one time a second instance is wanted.
    if config::port_override().is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            eprintln!("[app] already running — showing the existing window");
            reveal(app);
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    let action = {
                        let list = match bindings_for_handler.lock() {
                            Ok(l) => l,
                            Err(e) => e.into_inner(),
                        };
                        list.iter()
                            .find(|(s, _)| s == shortcut)
                            .map(|(_, a)| a.clone())
                    };
                    let Some(action) = action else { return };
                    let pressed = matches!(event.state(), ShortcutState::Pressed);
                    keys::dispatch(app, &ptt_for_handler, action, pressed);
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::net_start,
            commands::net_stop,
            commands::net_stats,
            commands::net_peers,
            commands::local_name,
            commands::config_get,
            commands::ptt_held,
            commands::send_text,
            commands::messages,
            commands::manual_peers,
            commands::set_target,
            commands::set_label,
            commands::audio_devices,
            commands::set_audio_devices,
            commands::set_shortcut,
            commands::set_order,
            commands::room_new,
            commands::room_code,
            commands::set_passphrase,
            keys::shortcuts,
        ])
        .setup(move |app| {
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Zetta Com")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => reveal(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        reveal(tray.app_handle());
                    }
                })
                .build(app)?;

            let cfg = config::load(app.handle()).unwrap_or_default();
            keys::register_all(&app.handle().clone(), &bindings, &listing, &cfg);
            app.manage(Ptt(ptt.clone()));
            app.manage(Shortcuts(listing.clone()));
            app.manage(BindingsState(bindings.clone()));

            // Auto-start from the saved settings. A PC whose job is to listen
            // must come up receiving without anyone clicking anything: it may
            // be in another room, it may have no microphone, and it may have
            // nobody sitting at it. Requiring a button press made a listener
            // depend on a person, which is the one thing v1 got right.
            //
            // Settings come from disk rather than the webview's localStorage,
            // because binding has to happen before any window has loaded, and
            // from a file rather than the environment, because two ways to
            // configure one thing guarantees an afternoon spent on "why is it
            // using the other port".
            //
            // The port is resolved once and then reported: a log that
            // misreports which port is in use is worse than no log.
            let port = config::port_override().unwrap_or(cfg.port);
            let session = match session::start(
                port,
                &cfg.peer,
                &cfg.manual,
                cfg.labels.clone(),
                cfg.order.clone(),
                cfg.passphrase.clone(),
                commands::audio_prefs(&cfg),
                ptt.clone(),
            ) {
                Ok(s) => {
                    eprintln!("[net] auto-started on {port} -> {}", cfg.peer);
                    Some(s)
                }
                Err(e) => {
                    // Not fatal: the window still opens, stopped, so the
                    // settings can be corrected manually.
                    eprintln!("[net] auto-start failed: {e:#}");
                    None
                }
            };
            app.manage(NetState(Mutex::new(session)));

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide rather than quit: the intercom has to keep receiving
                // while its window is shut. Quit lives in the tray menu, and
                // is the only way out once this is in place.
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
