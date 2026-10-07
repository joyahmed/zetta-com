//! The window and Rust describe the same structs twice, in two languages, and
//! nothing connects them: `src/types.d.ts` is typed by hand against whatever
//! `serde` happens to send. Rename a field on one side and `tsc` stays green,
//! the build stays green, and the screen quietly shows `undefined`.
//!
//! This test is the connection. For every struct that crosses to the window it
//! builds a sample, serialises it exactly as the IPC layer does, and compares
//! the top-level keys with the field names of the matching `type X = { ... }`
//! block in `types.d.ts`. A mismatch fails with the type name and the field.
//!
//! It replaces adopting `tauri-specta` (docs/IMPROVEMENTS.md, Phase F): that
//! crate is a release candidate on Tauri 2 with no stable release, and this
//! catches the drift that actually happens — a renamed or forgotten field —
//! with no new dependency. It checks names, not value types.

use crate::{config, discovery, keys, net, state};
use std::collections::BTreeSet;

const TYPES: &str = include_str!("../../src/types.d.ts");

/// Field names of `type <name> = { ... }`: the one-line form
/// (`{ a: string; b: number }`) or the block form with one field per line at a
/// single tab of indent. `///` comments, blank lines and deeper-indented
/// continuation lines are skipped. Panics with a plain message when the type is
/// missing, because a test that finds nothing to compare must not pass.
fn ts_fields(name: &str) -> BTreeSet<String> {
    let head = format!("type {name} = {{");
    let mut lines = TYPES.lines();
    let first = lines
        .by_ref()
        .find(|l| l.starts_with(&head))
        .unwrap_or_else(|| panic!("types.d.ts has no `type {name} = {{ ... }}` block"));

    if let Some(inline) = first[head.len()..].strip_suffix("};") {
        let inline = inline.trim_end().trim_end_matches('}');
        return inline.split(';').filter_map(field_name).collect();
    }

    let mut out = BTreeSet::new();
    for l in lines {
        if l.starts_with("};") {
            return out;
        }
        // Exactly one tab: a field of this type, not of something nested in it.
        if let Some(rest) = l.strip_prefix('\t') {
            if !rest.starts_with(['\t', '/', ' ']) {
                out.extend(field_name(rest));
            }
        }
    }
    panic!("types.d.ts: `type {name}` block never closes with `}};`");
}

/// `label: string` or `nobody?: boolean` -> `label` / `nobody`.
fn field_name(piece: &str) -> Option<String> {
    let piece = piece.trim();
    let end = piece.find(':')?;
    let name = piece[..end].trim_end_matches('?').trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Field names of the `| { kind: '<kind>'; ... }` member of a union type.
fn ts_variant_fields(name: &str, kind: &str) -> BTreeSet<String> {
    let head = format!("type {name} =");
    let mut lines = TYPES.lines().skip_while(|l| !l.starts_with(&head));
    assert!(lines.next().is_some(), "types.d.ts has no `type {name} =` union");
    let tag = format!("kind: '{kind}'");
    for l in lines {
        let l = l.trim();
        if let Some(body) = l.strip_prefix("| {") {
            if body.contains(&tag) {
                let body = body.trim_end_matches(';').trim_end_matches('}');
                return body.split(';').filter_map(field_name).collect();
            }
        }
        if l.ends_with(';') && !l.starts_with('|') {
            break;
        }
    }
    panic!("types.d.ts: union `{name}` has no member with {tag}");
}

fn rust_keys<T: serde::Serialize>(sample: &T) -> BTreeSet<String> {
    match serde_json::to_value(sample).unwrap() {
        serde_json::Value::Object(m) => m.keys().cloned().collect(),
        other => panic!("expected an object, got {other}"),
    }
}

/// `only_rust` / `only_ts`: fields one side deliberately lacks, each with its
/// reason at the call site.
fn check(name: &str, rust: BTreeSet<String>, ts: BTreeSet<String>, only_rust: &[&str], only_ts: &[&str]) {
    let strip = |set: &BTreeSet<String>, skip: &[&str]| -> BTreeSet<String> {
        set.iter().filter(|k| !skip.contains(&k.as_str())).cloned().collect()
    };
    let rust = strip(&rust, only_rust);
    let ts = strip(&ts, only_ts);
    let missing_in_ts: Vec<_> = rust.difference(&ts).collect();
    let missing_in_rust: Vec<_> = ts.difference(&rust).collect();
    assert!(
        missing_in_ts.is_empty() && missing_in_rust.is_empty(),
        "IPC drift in `{name}`: Rust sends {missing_in_ts:?} that types.d.ts does not declare; \
         types.d.ts declares {missing_in_rust:?} that Rust never sends"
    );
}

#[test]
fn peer_matches() {
    let sample = discovery::Peer {
        id: "a".into(),
        name: "A".into(),
        addr: "10.0.0.1:9001".parse().unwrap(),
        live: true,
        talking: false,
        manual: false,
        version: None,
        busy: false,
    };
    check("Peer", rust_keys(&sample), ts_fields("Peer"), &[], &[]);
}

#[test]
fn message_matches() {
    let sample = net::Message {
        id: 1,
        from: String::new(),
        text: String::new(),
        mine: true,
        at: 0,
        wire: 0,
        to: vec![],
        heard: vec![],
        waiting: vec![],
    };
    // `wire` is `#[serde(skip)]`, so it is not in the JSON either way.
    check("Message", rust_keys(&sample), ts_fields("Message"), &[], &[]);
}

#[test]
fn stats_matches() {
    let sample = net::Stats {
        port: 0,
        peer: String::new(),
        tx: 0,
        rx: 0,
        bad: 0,
        lost: 0,
        last_seq: 0,
    };
    // The window calls its copy `NetStats`.
    check("Stats", rust_keys(&sample), ts_fields("NetStats"), &[], &[]);
}

#[test]
fn config_matches() {
    check(
        "Config",
        rust_keys(&config::Config::default()),
        ts_fields("Config"),
        // The UI never reads these: names for machines and the shortcut and
        // roster-order tables are set through their own commands and shown
        // from the roster / shortcut list, not from the config object.
        &["labels", "shortcuts", "order"],
        &[],
    );
}

#[test]
fn sounds_matches() {
    check("Sounds", rust_keys(&config::Sounds::default()), ts_fields("Sounds"), &[], &[]);
}

#[test]
fn group_matches() {
    let sample = config::Group { id: String::new(), name: String::new(), members: vec![] };
    check("Group", rust_keys(&sample), ts_fields("Group"), &[], &[]);
}

#[test]
fn preset_matches() {
    let sample = config::Preset { label: String::new(), text: String::new(), shortcut: String::new() };
    check("Preset", rust_keys(&sample), ts_fields("Preset"), &[], &[]);
}

#[test]
fn shortcut_info_matches() {
    let sample = keys::ShortcutInfo {
        label: String::new(),
        keys: String::new(),
        registered: true,
        id: None,
    };
    check("ShortcutInfo", rust_keys(&sample), ts_fields("ShortcutInfo"), &[], &[]);
}

/// `Target` is an enum, serialised as `{ kind, ...fields }`, and `types.d.ts`
/// declares it as a union. Each Rust variant is compared with the union member
/// that carries the same `kind`.
#[test]
fn target_variants_match() {
    use state::Target;
    let cases = [
        ("everyone", Target::Everyone),
        ("pc", Target::Pc { addr: String::new() }),
        ("group", Target::Group { id: String::new(), name: String::new() }),
        ("gone", Target::Gone { name: String::new() }),
    ];
    for (kind, sample) in cases {
        check(
            &format!("Target::{kind}"),
            rust_keys(&sample),
            ts_variant_fields("Target", kind),
            &[],
            &[],
        );
    }
}
