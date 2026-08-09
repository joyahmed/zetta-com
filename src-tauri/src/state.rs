//! Shared application state.
//!
//! Both the commands and the shortcut handler reach for these, so they live
//! apart from either — a module that owns the nouns rather than the verbs.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::session;

pub struct NetState(pub Mutex<Option<session::Session>>);

/// True only while the push-to-talk key is held.
///
/// It lives outside the session and outlives it, because the shortcut is
/// registered once at launch: the key must not stop working because somebody
/// pressed Stop and Start.
pub struct Ptt(pub Arc<AtomicBool>);

