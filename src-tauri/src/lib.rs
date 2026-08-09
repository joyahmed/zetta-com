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
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

use keys::{reveal, toggle, Bindings, BindingsState, ShortcutList, Shortcuts};
use state::{NetState, Ptt};

/// Bring the transport up from the saved settings, or report why it could not.
///
/// `None` is not fatal: the window still opens, stopped, so whatever is wrong
/// can be corrected by hand. Shared by the launch path and the delayed one so
/// there is one description of what auto-start means.
fn bring_up(
    port: u16,
    cfg: &config::Config,
    ptt: &Arc<AtomicBool>,
) -> Option<session::Session> {
    match session::start(
        port,
        &cfg.peer,
        &cfg.manual,
        cfg.labels.clone(),
        cfg.order.clone(),
        cfg.passphrase.clone(),
        commands::audio_prefs(cfg),
        ptt.clone(),
    ) {
        Ok(s) => {
            eprintln!("[net] auto-started on {port} -> {}", cfg.peer);
            Some(s)
        }
        Err(e) => {
            eprintln!("[net] auto-start failed: {e:#}");
            None
        }
    }
}

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
        // Start with the machine, off, in the tray. An intercom that has to be
        // launched is one that is not listening when somebody calls you — and
        // the person calling gets silence, which looks exactly like being
        // ignored. The flag is what tells the app it was Windows that started
        // it and not a person; see `config::autostarted`.
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![config::AUTOSTART_FLAG]),
        ))
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
            commands::set_start_delay,
            keys::shortcuts,
        ])
        .setup(move |app| {
            // The window is configured hidden, and revealed here — before
            // anything that can fail, so a broken tray leaves a visible window
            // rather than a process with no way to reach it.
            //
            // It is configured that way because the alternative is worse on the
            // one launch that matters: a login start would paint the whole
            // window, then hide it a moment later, so every boot would flash the
            // app on screen and take focus off whatever the user was doing.
            let autostarted = config::autostarted();
            if !autostarted {
                reveal(app.handle());
            } else {
                eprintln!("[app] started by Windows — staying in the tray");
            }

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
                // Left click toggles, the way a tray icon is expected to: it is
                // the same one gesture that put the window away. Focus is not
                // consulted here — clicking the tray has already taken focus off
                // the window, so asking would answer "not focused" every time
                // and the icon could only ever open. The menu's Show stays a
                // plain show: an item that says Show and hides is a lie.
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle(tray.app_handle(), false);
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

            // On a login start, wait before binding — see `config::start_delay`.
            // Windows runs the Run key while it is still bringing the network
            // up, so binding immediately fails against an adapter that has no
            // address yet, and mDNS advertises on a stack that is not there.
            //
            // The state is managed empty first and filled from the thread,
            // because `manage` can only be called once and the commands must
            // exist from the moment the window loads either way.
            let delay = if autostarted { cfg.start_delay } else { 0 };
            if delay == 0 {
                app.manage(NetState(Mutex::new(bring_up(port, &cfg, &ptt))));
            } else {
                app.manage(NetState(Mutex::new(None)));
                eprintln!("[net] waiting {delay}s for the network before binding");
                let handle = app.handle().clone();
                let ptt = ptt.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(delay));
                    // Re-read rather than reusing the config from launch: the
                    // window has been up for the whole wait, and the port may
                    // have been changed in it.
                    let cfg = config::load(&handle).unwrap_or_default();
                    let port = config::port_override().unwrap_or(cfg.port);
                    let state = handle.state::<NetState>();
                    let mut guard = match state.0.lock() {
                        Ok(g) => g,
                        Err(e) => e.into_inner(),
                    };
                    // Somebody pressed Start during the wait. Binding now would
                    // drop a transport that is already working and rebind the
                    // same port, which is the one thing this delay exists to
                    // avoid doing at a moment nobody is watching.
                    if guard.is_some() {
                        return eprintln!("[net] already started by hand — not auto-starting");
                    }
                    *guard = bring_up(port, &cfg, &ptt);
                });
            }

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
