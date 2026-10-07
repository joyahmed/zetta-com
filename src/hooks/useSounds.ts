import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { DEMO } from '../demo';
import { setSoundPrefs, sound } from '../utils/sounds';

const DEFAULTS: Sounds = {
	message: true,
	voice: true,
	key: false,
	presence: true,
	volume: 0.7
};

/// How long after a session starts that presence changes stay silent. A fresh
/// session has heard nobody yet and heartbeats come every 2 s, so for two
/// beats plus margin "not live" means "not heard yet", not "went away".
export const SETTLE_MS = 4500;

/// Decides the presence tone for one roster poll, or null for none.
///
/// `seen` is the set of watched addresses that have been live at least once
/// since this session started. It is updated here (after the decision), so a
/// PC's first sighting never plays "online" and a PC never heard at all can
/// never play "offline". While not `settled` it only learns, never plays.
/// One tone per poll, and offline wins over online. A PC missing from either
/// roster counts as not live.
export const presenceChange = (
	prevLive: Map<string, boolean>,
	nowLive: Map<string, boolean>,
	seen: Set<string>,
	settled: boolean,
	watched: string[]
): 'offline' | 'online' | null => {
	const live = (m: Map<string, boolean>, a: string) => m.get(a) ?? false;
	let gone = false;
	let back = false;
	for (const a of watched) {
		const was = live(prevLive, a);
		const is = live(nowLive, a);
		if (was !== is && seen.has(a)) {
			if (is) back = true;
			else gone = true;
		}
	}
	for (const a of watched) if (live(nowLive, a)) seen.add(a);
	if (!settled) return null;
	return gone ? 'offline' : back ? 'online' : null;
};

/// The sound settings, the per-PC watch list, and the listeners that play the
/// sounds Rust announces.
///
/// The voice and key sounds arrive as events because a poll would play them
/// late. Presence does not need one: "offline" already means seven seconds
/// without a heartbeat, so the 400 ms roster poll adds nothing anyone hears.
export const useSounds = (
	peers: Peer[],
	running: boolean,
	onError: (m: string) => void
) => {
	const [sounds, setSounds] = useState<Sounds>(DEFAULTS);
	const [watched, setWatched] = useState<string[]>([]);

	useEffect(() => {
		if (DEMO) return;
		invoke<Config | null>('config_get')
			.then(c => {
				if (!c) return;
				if (c.sounds) setSounds(c.sounds);
				setWatched(c.watched ?? []);
			})
			.catch(() => {});
	}, []);

	useEffect(() => setSoundPrefs(sounds), [sounds]);

	useEffect(() => {
		const pending = [
			listen('voice-start', () => sound('voice')),
			listen<boolean>('talk-key', e => sound(e.payload ? 'keyDown' : 'keyUp')),
			// Rust restarted the session in place (passphrase, audio device or PC
			// list changed): `running` never flips, so reset the presence baseline.
			listen('session-restart', () => resetPresence())
		];
		return () => {
			for (const p of pending) p.then(f => f()).catch(() => {});
		};
	}, []);

	// Who was live at the last poll. Empty until the first poll after a start,
	// so starting the transport does not announce everybody coming online, and
	// cleared on stop, so stopping does not announce everybody leaving.
	// `seen` and `startedAt` are the same idea for a fresh session: see
	// presenceChange.
	const was = useRef<Map<string, boolean> | null>(null);
	const seen = useRef(new Set<string>());
	const startedAt = useRef(0);
	const resetPresence = () => {
		was.current = null;
		seen.current.clear();
		startedAt.current = Date.now();
	};
	useEffect(() => {
		if (running) resetPresence();
		else was.current = null;
	}, [running]);
	useEffect(() => {
		if (!running) return;
		const now = new Map(peers.map(p => [p.addr, p.live]));
		const before = was.current;
		was.current = now;
		if (!before) return;
		const settled = Date.now() - startedAt.current >= SETTLE_MS;
		const tone = presenceChange(before, now, seen.current, settled, watched);
		if (tone) sound(tone);
	}, [peers, running, watched]);

	const choose = async (next: Partial<Sounds>) => {
		const merged = { ...sounds, ...next };
		setSounds(merged);
		if (DEMO) return;
		try {
			await invoke('set_sounds', { sounds: merged });
		} catch (e) {
			onError(String(e));
		}
	};

	const watch = async (addr: string, on: boolean) => {
		setWatched(w => (on ? [...w.filter(a => a !== addr), addr] : w.filter(a => a !== addr)));
		if (DEMO) return;
		try {
			setWatched(await invoke<string[]>('set_watched', { addr, on }));
		} catch (e) {
			onError(String(e));
		}
	};

	return { sounds, choose, watched, watch };
};
