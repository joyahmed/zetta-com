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

/// The tray's "Do not disturb" check item, kept so that switching DND from the
/// window can tick or untick it. Without this the tray and the window would
/// each show their own idea of the state, and the tray is the one people trust.
pub struct DndItem(pub tauri::menu::CheckMenuItem<tauri::Wry>);


/// Who you are talking to, as the window shows it.
///
/// The description, not the addresses: `net::Aim` is what the socket needs,
/// and this is what a person chose. They differ for a group — the choice is
/// "Devs", the addresses are whoever is in Devs right now — and keeping the
/// choice is what lets an edit to the group reach the aim without anybody
/// picking it again.
///
/// Sent to the window as `{ kind: "everyone" }`, `{ kind: "pc", addr }`,
/// `{ kind: "group", id, name }` or `{ kind: "gone", name }`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    #[default]
    Everyone,
    Pc { addr: String },
    Group { id: String, name: String },
    /// The group that was aimed at has been deleted. Sends go to nobody until
    /// something else is picked — see `net::pick` for why never to everyone —
    /// and the name is kept so the window can say which group it was.
    Gone { name: String },
}

/// The aim, owned by the app rather than by a session.
///
/// It used to live inside the transport, created as "everyone" each time one
/// was built. So every restart — a passphrase, a microphone, a PC added by
/// hand — silently re-aimed at the whole room, and choosing someone while
/// stopped did nothing at all, then Start sent to everyone. Now there is one
/// lock for the life of the app, handed to each transport as it is built, and
/// setting it while stopped is simply stored.
///
/// `aim` is what the socket reads on every send; `target` is what was chosen.
/// Both are written only by `commands::apply_target`, so they cannot drift.
pub struct TargetState {
    pub target: Mutex<Target>,
    pub aim: Arc<Mutex<crate::net::Aim>>,
}

impl TargetState {
    pub fn new() -> Self {
        Self {
            target: Mutex::new(Target::Everyone),
            aim: Arc::new(Mutex::new(crate::net::Aim::All)),
        }
    }
}

/// Turn a choice into addresses, against the groups as they are saved now.
///
/// Returns the choice too, because resolving can change it: a renamed group
/// carries its new name, and a deleted one becomes `Gone`. A member address
/// that does not parse is skipped rather than fatal — one stale entry must not
/// make the whole group unreachable.
///
/// Pure, so the rules can be tested without an app.
pub fn resolve(target: &Target, groups: &[crate::config::Group]) -> (Target, crate::net::Aim) {
    use crate::net::Aim;
    match target {
        Target::Everyone => (Target::Everyone, Aim::All),
        Target::Pc { addr } => match addr.parse() {
            Ok(a) => (target.clone(), Aim::One(a)),
            // Only reachable from a hand-written call: the setter resolves
            // before storing. Nobody, by the same rule as a missing group.
            Err(_) => (target.clone(), Aim::Nobody),
        },
        Target::Group { id, name } => match groups.iter().find(|g| g.id == *id) {
            Some(g) => (
                Target::Group {
                    id: g.id.clone(),
                    name: g.name.clone(),
                },
                Aim::Group(
                    g.members
                        .iter()
                        .filter_map(|m| m.parse().ok())
                        .collect(),
                ),
            ),
            None => (Target::Gone { name: name.clone() }, Aim::Nobody),
        },
        Target::Gone { .. } => (target.clone(), Aim::Nobody),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Group;
    use crate::net::Aim;

    fn devs(name: &str) -> Group {
        Group {
            id: "g1".into(),
            name: name.into(),
            members: vec!["10.0.0.1:9001".into(), "10.0.0.2:9001".into()],
        }
    }

    fn aimed_at_devs() -> Target {
        Target::Group {
            id: "g1".into(),
            name: "Devs".into(),
        }
    }

    #[test]
    fn renaming_the_aimed_group_keeps_the_aim() {
        let (t, aim) = resolve(&aimed_at_devs(), &[devs("Dev team")]);
        assert_eq!(
            t,
            Target::Group {
                id: "g1".into(),
                name: "Dev team".into()
            }
        );
        assert_eq!(
            aim,
            Aim::Group(vec![
                "10.0.0.1:9001".parse().unwrap(),
                "10.0.0.2:9001".parse().unwrap()
            ])
        );
    }

    #[test]
    fn deleting_the_aimed_group_aims_at_nobody_not_everyone() {
        let (t, aim) = resolve(&aimed_at_devs(), &[]);
        assert_eq!(t, Target::Gone { name: "Devs".into() });
        assert_eq!(aim, Aim::Nobody);
        // And it stays that way: saving again does not bring everyone back.
        assert_eq!(resolve(&t, &[devs("Devs")]).1, Aim::Nobody);
    }

    #[test]
    fn editing_members_reaches_the_aim_without_picking_again() {
        let mut g = devs("Devs");
        g.members = vec!["10.0.0.9:9001".into(), "not an address".into()];
        let (_, aim) = resolve(&aimed_at_devs(), &[g]);
        assert_eq!(aim, Aim::Group(vec!["10.0.0.9:9001".parse().unwrap()]));
    }
}
